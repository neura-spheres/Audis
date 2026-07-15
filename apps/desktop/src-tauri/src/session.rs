//! The live session pipeline.
//!
//! This is the path a spoken word takes to the screen:
//!
//! ```text
//! capture (48 kHz, stereo, device thread)
//!   -> frame channel (bounded, never blocks the audio thread)
//!   -> prepare thread: downmix -> stateful resample to 16 kHz -> endpointer
//!   -> utterance channel
//!   -> recognise thread: Whisper -> TRANSCRIPT_FINAL
//! ```
//!
//! Each source gets its own capture and prepare thread, but both feed one
//! recognise thread. That is deliberate: a Whisper context is large and
//! CPU-bound, so two would double memory and fight for the same cores.
//! Attribution survives because the utterance carries its source, which is the
//! device it arrived on rather than a guess.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use audis_asr::{AsrEngine, EndpointConfig, EndpointEvent, Endpointer, Resampler, downmix_to_mono};
use audis_common::{
    AsrState, AsrStatus, AudioSourceKind, AudisError, DiagnosticWarning, Language, Result,
    SessionMode, SessionState, SessionStatus, TranscriptSegment, events,
};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

/// How many frame blocks may wait for the prepare thread.
///
/// Roughly a second of audio. If the prepare thread ever fell this far behind,
/// dropping the oldest audio is better than growing without bound or, far
/// worse, blocking the real-time audio callback.
const FRAME_QUEUE_DEPTH: usize = 64;

/// How many utterances may wait to be recognised.
///
/// Whisper is roughly real-time, so this only fills if the machine is briefly
/// overloaded. It is deep enough to ride that out and shallow enough that the
/// user never sees minutes-stale captions.
const UTTERANCE_QUEUE_DEPTH: usize = 32;

/// How often an in-progress sentence is offered for an interim caption.
///
/// Not how often one is decoded: interim work only happens when the recogniser
/// has nothing final waiting, so on a loaded machine these are simply skipped.
/// Fast enough to feel live, slow enough not to spend the whole CPU budget
/// re-decoding the same words.
const PARTIAL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(600);

/// What to start, gathered from settings by the caller.
pub struct SessionRequest {
    /// The engine that will recognise speech, already built and ready.
    ///
    /// Built by the caller rather than here: choosing between a local model and
    /// a provider needs settings and the keystore, and the pipeline downstream
    /// is identical either way.
    pub engine: Box<dyn AsrEngine>,
    /// Where sessions are stored, when this mode saves one.
    pub paths: audis_common::AppPaths,
    /// Which feature is running.
    pub mode: SessionMode,
    /// The language to recognise. Always set, never detected.
    pub language: Language,
    /// Microphone endpoint, or the Windows default.
    pub microphone_id: Option<String>,
    /// Output endpoint to capture via loopback, or the Windows default.
    pub computer_audio_id: Option<String>,
    /// Capture the microphone.
    pub want_microphone: bool,
    /// Capture what the computer is playing.
    pub want_computer_audio: bool,
}

/// One utterance, tagged with where it came from.
struct SourcedUtterance {
    source: AudioSourceKind,
    utterance: audis_asr::Utterance,
}

/// The newest sentence-in-progress waiting for an interim decode.
///
/// A single slot rather than a queue, and deliberately so: an interim caption
/// is only worth showing if it is the latest thing said. A queue would decode
/// stale audio and show words the speaker has already moved past. Newest wins,
/// older ones are simply dropped.
type PartialSlot = Arc<Mutex<Option<SourcedUtterance>>>;

/// Everything a running session owns.
struct Running {
    id: Uuid,
    mode: SessionMode,
    language: Language,
    state: SessionState,
    /// Captured milliseconds from runs already ended by a pause.
    captured_ms: u64,
    /// When the current capturing run began, or `None` while paused.
    running_since: Option<std::time::Instant>,
    /// Set to stop every worker thread.
    stop: Arc<AtomicBool>,
    /// True while paused. Audio keeps flowing and is discarded, which keeps the
    /// device open so resuming is instant.
    paused: Arc<AtomicBool>,
    /// Dropping these stops capture.
    captures: Vec<audis_audio::CaptureHandle>,
    workers: Vec<JoinHandle<()>>,
    microphone: bool,
    computer_audio: bool,
}

/// The session, as the commands see it.
#[derive(Default)]
pub struct SessionController {
    inner: Mutex<Option<Running>>,
}

impl SessionController {
    /// Start a session.
    ///
    /// The model is loaded before any device is opened, so a missing model
    /// fails cleanly with nothing half-started.
    pub fn start(&self, app: &AppHandle, request: SessionRequest) -> Result<SessionStatus> {
        let SessionRequest {
            engine,
            paths,
            mode,
            language,
            microphone_id,
            computer_audio_id,
            want_microphone,
            want_computer_audio,
        } = request;

        let mut guard = self.lock();

        if guard
            .as_ref()
            .is_some_and(|running| running.state.is_active())
        {
            return Err(AudisError::InvalidArgument {
                field: "feature".to_owned(),
                detail: "a session is already running".to_owned(),
            });
        }

        if !want_microphone && !want_computer_audio {
            return Err(AudisError::InvalidArgument {
                field: "transcription.captureMicrophone".to_owned(),
                detail: "a session needs at least one audio source".to_owned(),
            });
        }

        let id = Uuid::new_v4();
        emit_state(
            app,
            id,
            mode,
            language,
            SessionState::Starting,
            0,
            false,
            false,
        );

        // Live Caption promises nothing is written to disk, so no writer is
        // constructed for it at all. Opened before any device so a disk failure
        // leaves no capture running.
        let writer = if mode.persists_transcript() {
            Some(crate::transcript_store::SessionWriter::create(
                &paths, id, mode, language,
            )?)
        } else {
            None
        };

        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));
        let partial: PartialSlot = Arc::new(Mutex::new(None));
        let (utterance_tx, utterance_rx) = sync_channel::<SourcedUtterance>(UTTERANCE_QUEUE_DEPTH);

        let mut captures = Vec::new();
        let mut workers = Vec::new();

        // The recogniser owns the engine and serialises both sources.
        workers.push(
            spawn_recogniser(RecogniserSetup {
                app: app.clone(),
                session_id: id,
                language,
                engine,
                utterances: utterance_rx,
                partial: Arc::clone(&partial),
                stop: Arc::clone(&stop),
                writer,
            })
            .map_err(|error| AudisError::Configuration {
                detail: format!("the recognition thread could not start: {error}"),
            })?,
        );

        let mut started_microphone = false;
        let mut started_computer_audio = false;

        if want_microphone {
            match start_source(
                app,
                AudioSourceKind::Microphone,
                microphone_id.as_deref(),
                utterance_tx.clone(),
                Arc::clone(&partial),
                Arc::clone(&stop),
                Arc::clone(&paused),
            ) {
                Ok((capture, worker)) => {
                    captures.push(capture);
                    workers.push(worker);
                    started_microphone = true;
                }
                Err(error) => {
                    // One source failing must not kill the other: a missing
                    // microphone should still let you caption a video.
                    tracing::warn!(%error, "microphone capture could not start");
                    emit_source_failed(app, AudioSourceKind::Microphone, &error);
                }
            }
        }

        if want_computer_audio {
            match start_source(
                app,
                AudioSourceKind::ComputerAudio,
                computer_audio_id.as_deref(),
                utterance_tx.clone(),
                Arc::clone(&partial),
                Arc::clone(&stop),
                Arc::clone(&paused),
            ) {
                Ok((capture, worker)) => {
                    captures.push(capture);
                    workers.push(worker);
                    started_computer_audio = true;
                }
                Err(error) => {
                    tracing::warn!(%error, "computer audio capture could not start");
                    emit_source_failed(app, AudioSourceKind::ComputerAudio, &error);
                }
            }
        }

        // Dropping the last sender is what tells the recogniser to finish, so
        // the template copy must not outlive the sources.
        drop(utterance_tx);

        if !started_microphone && !started_computer_audio {
            stop.store(true, Ordering::SeqCst);
            for worker in workers {
                let _ = worker.join();
            }
            emit_state(
                app,
                id,
                mode,
                language,
                SessionState::Failed,
                0,
                false,
                false,
            );

            return Err(AudisError::Configuration {
                detail: "no audio source could be opened. Check Audio settings.".to_owned(),
            });
        }

        let status = SessionStatus {
            id,
            mode,
            state: SessionState::Listening,
            language,
            elapsed_ms: 0,
            microphone: started_microphone,
            computer_audio: started_computer_audio,
            captions_visible: true,
            assistant_enabled: mode.uses_cloud_ai(),
            error: None,
        };

        *guard = Some(Running {
            id,
            mode,
            language,
            state: SessionState::Listening,
            captured_ms: 0,
            running_since: Some(std::time::Instant::now()),
            stop,
            paused,
            captures,
            workers,
            microphone: started_microphone,
            computer_audio: started_computer_audio,
        });

        show_captions(app, true);
        app.emit(events::SESSION_STATE, &status).ok();
        tracing::info!(%id, ?mode, "session started");

        Ok(status)
    }

    /// Pause or resume. Audio keeps flowing while paused and is discarded, so
    /// the device stays open and resuming is instant.
    pub fn set_paused(&self, app: &AppHandle, pause: bool) -> Result<SessionStatus> {
        let mut guard = self.lock();
        let running = guard.as_mut().ok_or_else(no_session)?;

        let next = if pause {
            SessionState::Paused
        } else {
            SessionState::Listening
        };

        if !running.state.can_transition_to(next) {
            return Err(AudisError::InvalidArgument {
                field: "paused".to_owned(),
                detail: format!("cannot go from {:?} to {next:?}", running.state),
            });
        }

        running.paused.store(pause, Ordering::SeqCst);
        running.set_clock_running(!pause);
        running.state = next;

        let status = running.status();
        app.emit(events::SESSION_STATE, &status).ok();
        Ok(status)
    }

    /// Stop the session and release every device.
    pub fn stop(&self, app: &AppHandle) -> Result<SessionStatus> {
        let mut guard = self.lock();
        let Some(mut running) = guard.take() else {
            return Err(no_session());
        };

        let id = running.id;
        let mode = running.mode;
        let language = running.language;
        let elapsed = running.elapsed_ms();

        emit_state(
            app,
            id,
            mode,
            language,
            SessionState::Stopping,
            elapsed,
            running.microphone,
            running.computer_audio,
        );

        running.stop.store(true, Ordering::SeqCst);
        // Dropping the captures closes the frame channels, which is what ends
        // the prepare threads, which ends the recogniser.
        running.captures.clear();

        for worker in running.workers.drain(..) {
            let _ = worker.join();
        }

        let status = SessionStatus {
            id,
            mode,
            state: SessionState::Completed,
            language,
            elapsed_ms: elapsed,
            microphone: running.microphone,
            computer_audio: running.computer_audio,
            captions_visible: false,
            assistant_enabled: false,
            error: None,
        };

        show_captions(app, false);
        app.emit(events::SESSION_STATE, &status).ok();
        tracing::info!(%id, elapsed_ms = elapsed, "session stopped");

        Ok(status)
    }

    /// The current session, if one is running.
    pub fn status(&self) -> Option<SessionStatus> {
        self.lock().as_ref().map(Running::status)
    }

    /// A poisoned lock means a worker panicked while holding it. The session is
    /// still recoverable: take the state back rather than propagating a panic
    /// into every later command.
    fn lock(&self) -> std::sync::MutexGuard<'_, Option<Running>> {
        self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::error!("session lock was poisoned; recovering");
            poisoned.into_inner()
        })
    }
}

impl Running {
    /// Milliseconds of captured audio, excluding time spent paused.
    fn elapsed_ms(&self) -> u64 {
        let current = self
            .running_since
            .map(|since| u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0);
        self.captured_ms.saturating_add(current)
    }

    /// Stop or start the capture clock, on pause and resume.
    fn set_clock_running(&mut self, run: bool) {
        match (run, self.running_since) {
            (false, Some(since)) => {
                self.captured_ms = self
                    .captured_ms
                    .saturating_add(u64::try_from(since.elapsed().as_millis()).unwrap_or(u64::MAX));
                self.running_since = None;
            }
            (true, None) => self.running_since = Some(std::time::Instant::now()),
            _ => {}
        }
    }

    fn status(&self) -> SessionStatus {
        SessionStatus {
            id: self.id,
            mode: self.mode,
            state: self.state,
            language: self.language,
            elapsed_ms: self.elapsed_ms(),
            microphone: self.microphone,
            computer_audio: self.computer_audio,
            captions_visible: true,
            assistant_enabled: self.mode.uses_cloud_ai(),
            error: None,
        }
    }
}

/// Open one capture and spawn the thread that prepares its audio.
fn start_source(
    app: &AppHandle,
    source: AudioSourceKind,
    device_id: Option<&str>,
    utterances: SyncSender<SourcedUtterance>,
    partial: PartialSlot,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
) -> std::result::Result<(audis_audio::CaptureHandle, JoinHandle<()>), audis_audio::AudioError> {
    let (frame_tx, frame_rx) = sync_channel::<Vec<f32>>(FRAME_QUEUE_DEPTH);
    let dropped = Arc::new(AtomicU64::new(0));

    let sink_dropped = Arc::clone(&dropped);
    let sink: audis_audio::FrameSink = Arc::new(move |data: &[f32]| {
        // This runs on the real-time audio thread. `try_send` never blocks, so
        // a stalled consumer costs audio rather than glitching the device for
        // every application on the machine.
        if frame_tx.try_send(data.to_vec()).is_err() {
            sink_dropped.fetch_add(1, Ordering::Relaxed);
        }
    });

    let capture = match source {
        AudioSourceKind::Microphone => audis_audio::start_microphone_with_sink(device_id, sink)?,
        AudioSourceKind::ComputerAudio => {
            audis_audio::start_computer_audio_with_sink(device_id, sink)?
        }
    };

    let worker = spawn_prepare(
        app.clone(),
        source,
        capture.sample_rate(),
        capture.channels(),
        frame_rx,
        utterances,
        partial,
        stop,
        paused,
        dropped,
    )
    .map_err(|error| audis_audio::AudioError::StreamStart {
        device: capture.device_name().to_owned(),
        detail: format!("the audio pipeline thread could not start: {error}"),
    })?;

    Ok((capture, worker))
}

/// Downmix, resample and endpoint one source's audio.
#[allow(clippy::too_many_arguments)]
fn spawn_prepare(
    app: AppHandle,
    source: AudioSourceKind,
    sample_rate: u32,
    channels: u16,
    frames: Receiver<Vec<f32>>,
    utterances: SyncSender<SourcedUtterance>,
    partial: PartialSlot,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("audis-prepare-{source:?}"))
        .spawn(move || {
            // One resampler for the whole session. It carries filter history
            // across blocks, which is what keeps the 50-per-second block seams
            // from becoming audible clicks and wrecking recognition.
            let mut resampler = Resampler::new(sample_rate);
            let mut endpointer = Endpointer::new(EndpointConfig::default());
            let mut reported_drops = 0u64;
            let mut last_partial = std::time::Instant::now();

            while let Ok(block) = frames.recv() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                // Discard while paused, but keep pulling frames so the channel
                // never backs up and the device stays open.
                if paused.load(Ordering::SeqCst) {
                    continue;
                }

                let mono = downmix_to_mono(&block, channels);
                let ready = resampler.push(&mono);
                if ready.is_empty() {
                    continue;
                }

                let event = endpointer.push(&ready);

                // Offer the sentence so far for an interim caption. Throttled,
                // and only ever an offer: the recogniser picks it up if it has
                // nothing final to do, and drops it otherwise.
                if matches!(
                    event,
                    EndpointEvent::SpeechStarted | EndpointEvent::Speaking
                ) && last_partial.elapsed() >= PARTIAL_INTERVAL
                    && let Some(in_progress) = endpointer.snapshot()
                {
                    last_partial = std::time::Instant::now();
                    if let Ok(mut slot) = partial.lock() {
                        *slot = Some(SourcedUtterance {
                            source,
                            utterance: in_progress,
                        });
                    }
                }

                if let EndpointEvent::Utterance(utterance) = event {
                    // A full queue means recognition is behind. Dropping the
                    // utterance keeps captions current instead of stalling
                    // capture to deliver minutes-old text.
                    if utterances
                        .try_send(SourcedUtterance { source, utterance })
                        .is_err()
                    {
                        tracing::warn!(?source, "recognition is behind; dropped an utterance");
                    }
                }

                let total = dropped.load(Ordering::Relaxed);
                if total > reported_drops {
                    reported_drops = total;
                    app.emit(
                        events::DIAGNOSTIC_WARNING,
                        DiagnosticWarning {
                            kind: "audio.frames_dropped".to_owned(),
                            message: format!(
                                "Audis could not keep up with {} audio and skipped some of it.",
                                source.default_label().to_lowercase()
                            ),
                        },
                    )
                    .ok();
                }
            }

            // Whatever was mid-sentence when the user pressed stop is still
            // worth transcribing.
            if let Some(utterance) = endpointer.flush() {
                let _ = utterances.try_send(SourcedUtterance { source, utterance });
            }

            tracing::debug!(?source, "prepare thread finished");
        })
}

/// What the recognise thread needs to run.
struct RecogniserSetup {
    app: AppHandle,
    session_id: Uuid,
    language: Language,
    engine: Box<dyn AsrEngine>,
    utterances: Receiver<SourcedUtterance>,
    /// Sentence-in-progress offered for an interim caption, if any.
    partial: PartialSlot,
    stop: Arc<AtomicBool>,
    /// `None` for a mode that promises not to write to disk.
    writer: Option<crate::transcript_store::SessionWriter>,
}

/// Own the engine and turn utterances into transcript events.
fn spawn_recogniser(setup: RecogniserSetup) -> std::io::Result<JoinHandle<()>> {
    let RecogniserSetup {
        app,
        session_id,
        language,
        mut engine,
        utterances,
        partial,
        stop,
        mut writer,
    } = setup;

    std::thread::Builder::new()
        .name("audis-recognise".to_owned())
        .spawn(move || {
            let engine_id = engine.id().to_owned();

            for source in [AudioSourceKind::Microphone, AudioSourceKind::ComputerAudio] {
                emit_asr_status(&app, source, AsrState::Listening, &engine_id, None);
            }

            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                // Finished sentences always win. An interim caption is a nicety;
                // a finished one is the product. So interim work only happens
                // with nothing final waiting, and on a machine that cannot keep
                // up the interim captions quietly stop while the real ones keep
                // arriving on time.
                match utterances.try_recv() {
                    Ok(SourcedUtterance { source, utterance }) => {
                        decode_final(
                            &app,
                            engine.as_mut(),
                            &engine_id,
                            session_id,
                            language,
                            source,
                            &utterance,
                            writer.as_mut(),
                        );
                        continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }

                // Idle: spend the time on the sentence being spoken right now.
                let waiting = partial.lock().ok().and_then(|mut slot| slot.take());
                if let Some(SourcedUtterance { source, utterance }) = waiting {
                    decode_partial(
                        &app,
                        engine.as_mut(),
                        session_id,
                        language,
                        source,
                        &utterance,
                    );
                    continue;
                }

                // Genuinely nothing to do. Block rather than spin, but wake
                // often enough to notice a new sentence starting.
                match utterances.recv_timeout(std::time::Duration::from_millis(40)) {
                    Ok(SourcedUtterance { source, utterance }) => decode_final(
                        &app,
                        engine.as_mut(),
                        &engine_id,
                        session_id,
                        language,
                        source,
                        &utterance,
                        writer.as_mut(),
                    ),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            if let Some(writer) = writer {
                let saved = writer.segment_count();
                match writer.finish() {
                    Ok(path) => tracing::info!(?path, saved, "transcript saved"),
                    Err(error) => tracing::error!(%error, "the transcript could not be closed"),
                }
            }

            for source in [AudioSourceKind::Microphone, AudioSourceKind::ComputerAudio] {
                emit_asr_status(&app, source, AsrState::Stopped, &engine_id, None);
            }

            tracing::debug!("recognise thread finished");
        })
}

/// Recognise a finished sentence, save it, and publish it.
///
/// This is the product: everything here is durable and is what the user reads
/// afterwards.
#[allow(clippy::too_many_arguments)]
fn decode_final(
    app: &AppHandle,
    engine: &mut dyn AsrEngine,
    engine_id: &str,
    session_id: Uuid,
    language: Language,
    source: AudioSourceKind,
    utterance: &audis_asr::Utterance,
    writer: Option<&mut crate::transcript_store::SessionWriter>,
) {
    emit_asr_status(app, source, AsrState::Recognising, engine_id, None);

    let result = match engine.transcribe(utterance, language) {
        Ok(result) => result,
        Err(error) => {
            // One bad utterance must not end the session.
            tracing::warn!(%error, ?source, "an utterance could not be recognised");
            emit_asr_status(
                app,
                source,
                AsrState::Listening,
                engine_id,
                Some(error.to_string()),
            );
            return;
        }
    };

    let segment = TranscriptSegment {
        id: Uuid::new_v4(),
        session_id,
        source,
        speaker: Some(source.default_label().to_owned()),
        start_ms: utterance.start_ms,
        end_ms: utterance.end_ms,
        text: result.text,
        language: result.language,
        confidence: result.confidence,
        is_final: true,
        engine: engine_id.to_owned(),
    };

    // Whisper emits "Thank you." and subtitle credits for silence. Publishing
    // that would put words in someone's mouth, so it is dropped rather than
    // shown.
    if segment.is_empty_speech() {
        tracing::debug!(text = %segment.text, "dropped a silence hallucination");
        emit_asr_status(app, source, AsrState::Listening, engine_id, None);
        return;
    }

    // Saved before it is shown. A failing disk should surface now rather than
    // after an hour of captions the user believed were being recorded. Captions
    // continue either way: losing the recording is bad, losing the live session
    // too would be worse.
    if let Some(Err(error)) = writer.map(|file| file.append(&segment)) {
        tracing::error!(%error, "a segment could not be saved");
        app.emit(
            events::DIAGNOSTIC_WARNING,
            DiagnosticWarning {
                kind: "transcript.write_failed".to_owned(),
                message:
                    "Audis could not save part of this transcript. Captions are still working."
                        .to_owned(),
            },
        )
        .ok();
    }

    app.emit(events::TRANSCRIPT_FINAL, &segment).ok();
    emit_asr_status(app, source, AsrState::Listening, engine_id, None);
}

/// Recognise a sentence still being spoken and publish it as interim text.
///
/// Deliberately does less than [`decode_final`]:
///
/// - It is **never written to disk.** This text will be superseded within the
///   second, and a transcript full of half-sentences would be worse than one
///   that waited. The saved transcript only ever contains finished sentences.
/// - It does **not** touch the ASR status, which would flicker between
///   Recognising and Listening several times a second.
/// - It still drops hallucinations, because the whole point is that a person is
///   reading this: showing invented words and retracting them is worse than
///   showing nothing for another moment.
fn decode_partial(
    app: &AppHandle,
    engine: &mut dyn AsrEngine,
    session_id: Uuid,
    language: Language,
    source: AudioSourceKind,
    utterance: &audis_asr::Utterance,
) {
    let Ok(result) = engine.transcribe(utterance, language) else {
        // Interim text is best-effort by definition. A failure here is not
        // worth a warning: the finished sentence will be along shortly.
        return;
    };

    let segment = TranscriptSegment {
        id: Uuid::new_v4(),
        session_id,
        source,
        speaker: Some(source.default_label().to_owned()),
        start_ms: utterance.start_ms,
        end_ms: utterance.end_ms,
        text: result.text,
        language: result.language,
        confidence: result.confidence,
        is_final: false,
        engine: "whisper-local".to_owned(),
    };

    if segment.is_empty_speech() {
        return;
    }

    // Interim captions are the one part of the pipeline with no lasting trace:
    // nothing is saved and the text is replaced within the second. Without a
    // line here there is no way to tell "interim captions are off" from
    // "interim captions are being skipped because the CPU is saturated".
    tracing::debug!(
        ?source,
        audio_ms = utterance.end_ms - utterance.start_ms,
        text = %segment.text,
        "interim caption"
    );

    app.emit(events::TRANSCRIPT_PARTIAL, &segment).ok();
}

fn emit_asr_status(
    app: &AppHandle,
    source: AudioSourceKind,
    state: AsrState,
    engine: &str,
    error: Option<String>,
) {
    app.emit(
        events::ASR_STATUS,
        AsrStatus {
            source,
            state,
            engine: engine.to_owned(),
            error,
        },
    )
    .ok();
}

fn emit_source_failed(app: &AppHandle, source: AudioSourceKind, error: &audis_audio::AudioError) {
    emit_asr_status(
        app,
        source,
        AsrState::Failed,
        "whisper",
        Some(error.to_string()),
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_state(
    app: &AppHandle,
    id: Uuid,
    mode: SessionMode,
    language: Language,
    state: SessionState,
    elapsed_ms: u64,
    microphone: bool,
    computer_audio: bool,
) {
    app.emit(
        events::SESSION_STATE,
        SessionStatus {
            id,
            mode,
            state,
            language,
            elapsed_ms,
            microphone,
            computer_audio,
            captions_visible: state.is_active(),
            assistant_enabled: mode.uses_cloud_ai(),
            error: None,
        },
    )
    .ok();
}

/// Show or hide the caption overlay.
///
/// Positioned along the bottom of the primary monitor on every show, because
/// the monitor layout can change between sessions and a caption bar stranded
/// off-screen is invisible with no way to find it.
fn show_captions(app: &AppHandle, show: bool) {
    let Some(window) = app.get_webview_window("captions") else {
        tracing::warn!("the captions window is missing from this build");
        return;
    };

    if !show {
        window.hide().ok();
        return;
    }

    if let Ok(Some(monitor)) = window.primary_monitor() {
        let screen = monitor.size();
        let scale = monitor.scale_factor();

        if let Ok(size) = window.outer_size() {
            // Signed maths first: on a monitor narrower than the caption bar
            // these would wrap to a huge positive number as u32 and strand the
            // window off-screen.
            let x = (i64::from(screen.width) - i64::from(size.width)) / 2;
            // A margin above the taskbar, scaled so it looks the same on a
            // high-DPI display.
            let margin = (72.0 * scale).round() as i64;
            let y = i64::from(screen.height) - i64::from(size.height) - margin;

            let x = u32::try_from(x.max(0)).unwrap_or(0);
            let y = u32::try_from(y.max(0)).unwrap_or(0);

            window.set_position(tauri::PhysicalPosition::new(x, y)).ok();
        }
    }

    window.show().ok();
    // Showing must not steal focus from whatever the user is watching.
    window.set_always_on_top(true).ok();

    // The window is reused across sessions, so re-apply the user's choice
    // rather than assuming it survived.
    let click_through = app
        .state::<crate::commands::AppState>()
        .settings
        .get()
        .captions
        .click_through;
    crate::commands::apply_caption_click_through(app, click_through);
}

fn no_session() -> AudisError {
    AudisError::InvalidArgument {
        field: "session".to_owned(),
        detail: "no session is running".to_owned(),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// A session with no device open reports nothing.
    #[test]
    fn a_fresh_controller_has_no_session() {
        let controller = SessionController::default();
        assert!(controller.status().is_none());
    }

    fn running(state: SessionState) -> Running {
        Running {
            id: Uuid::new_v4(),
            mode: SessionMode::LiveCaption,
            language: Language::English,
            state,
            captured_ms: 0,
            running_since: Some(std::time::Instant::now()),
            stop: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            captures: Vec::new(),
            workers: Vec::new(),
            microphone: true,
            computer_audio: false,
        }
    }

    /// `elapsed_ms` is documented as captured audio, excluding paused time. A
    /// meeting paused for a coffee break must not report the break as recorded
    /// audio: the number sits next to a transcript people trust.
    #[test]
    fn paused_time_does_not_count_as_captured_audio() {
        let mut session = running(SessionState::Listening);
        std::thread::sleep(std::time::Duration::from_millis(30));

        session.set_clock_running(false);
        let at_pause = session.elapsed_ms();
        assert!(at_pause >= 25, "expected the pre-pause audio to count");

        std::thread::sleep(std::time::Duration::from_millis(40));
        assert_eq!(
            session.elapsed_ms(),
            at_pause,
            "the clock must not advance while paused"
        );

        session.set_clock_running(true);
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(
            session.elapsed_ms() >= at_pause + 25,
            "the clock must resume where it left off"
        );
    }

    /// Pausing twice must not double-count or rewind the clock.
    #[test]
    fn pausing_twice_is_harmless() {
        let mut session = running(SessionState::Listening);
        std::thread::sleep(std::time::Duration::from_millis(20));

        session.set_clock_running(false);
        let first = session.elapsed_ms();
        session.set_clock_running(false);

        assert_eq!(session.elapsed_ms(), first);
    }

    #[test]
    fn status_reports_which_sources_are_live() {
        let session = running(SessionState::Listening);
        let status = session.status();

        assert!(status.microphone);
        assert!(!status.computer_audio);
        assert_eq!(status.state, SessionState::Listening);
    }

    /// Live Caption is local-only. Marking it assistant-enabled would imply
    /// text leaves the machine when it does not.
    #[test]
    fn a_local_only_mode_does_not_report_the_assistant_as_running() {
        let session = running(SessionState::Listening);
        assert!(!session.status().assistant_enabled);

        let mut meeting = running(SessionState::Listening);
        meeting.mode = SessionMode::MeetingAssistant;
        assert!(meeting.status().assistant_enabled);
    }
}
