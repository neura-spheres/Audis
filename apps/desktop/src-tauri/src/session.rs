//! The live session pipeline.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use audis_asr::{
    AsrEngine, Diarizer, EndpointConfig, EndpointEvent, Endpointer, LiveEvent, LiveInput,
    LiveOutput, Resampler, downmix_to_mono,
};
use audis_common::{
    AsrState, AsrStatus, AudioSourceKind, AudisError, DiagnosticWarning, Language, Result,
    SessionMode, SessionState, SessionStatus, SpeakerUpdate, TranscriptSegment, events,
};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// How many frame blocks may wait for the prepare thread.
const FRAME_QUEUE_DEPTH: usize = 64;

/// How many utterances may wait to be recognised.
const UTTERANCE_QUEUE_DEPTH: usize = 32;

/// How many frame blocks may wait for the recorder before one is dropped.
const RECORD_QUEUE_DEPTH: usize = 256;

/// How many things a live socket has said may wait for the recognise thread.
///
/// Deep on purpose: dropping one of these loses a line of transcript, and there
/// is no second copy of it anywhere.
const WORD_QUEUE_DEPTH: usize = 256;

/// How long a thread waits for work before checking whether to stop.
const POLL: std::time::Duration = std::time::Duration::from_millis(40);

/// How often an in-progress sentence is offered for an interim caption.
const PARTIAL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(400);

/// The silence gate a cloud engine ends an utterance on.
///
/// Shorter than the local gate: a large hosted model does not need the extra
/// context a local model does, so finals can be cut sooner and appear sooner.
const CLOUD_SILENCE_MS: u32 = 500;

/// How long a cloud engine lets one utterance run before cutting it anyway.
///
/// A cloud engine has no streaming API here: every request re-sends the whole
/// utterance as audio. Re-decoding a growing utterance for previews therefore
/// costs more and takes longer the longer someone talks. Cutting run-on speech
/// into short pieces instead means each piece is sent exactly once, small and
/// fast, so captions keep up without a preview and without the repeat traffic.
const CLOUD_MAX_UTTERANCE_MS: u32 = 4_000;

/// How long before the same recognition problem is worth mentioning again.
const PROBLEM_REPEAT_AFTER: std::time::Duration = std::time::Duration::from_secs(30);

/// Tells the user when recognition is failing, without repeating itself.
///
/// A cloud engine that is out of credit, rate limited, or holding a rejected key
/// fails on every chunk, which is several times a minute. Saying so once is
/// help; saying so every four seconds is a second fault on top of the first.
#[derive(Default)]
struct ProblemReporter {
    /// The last thing said, and when, so it is not said again immediately.
    last: Option<(String, std::time::Instant)>,
}

impl ProblemReporter {
    /// Report a failure, unless the same one was just reported.
    fn report(&mut self, app: &AppHandle, error: &audis_asr::AsrError) {
        let facing = error.to_user_facing();
        let message = format!("{} {}", facing.explanation, facing.suggested_action);

        let worth_saying = match &self.last {
            Some((said, at)) => *said != message || at.elapsed() >= PROBLEM_REPEAT_AFTER,
            None => true,
        };
        if !worth_saying {
            return;
        }

        self.last = Some((message.clone(), std::time::Instant::now()));
        app.emit(
            events::DIAGNOSTIC_WARNING,
            DiagnosticWarning {
                kind: "asr.failing".to_owned(),
                message,
            },
        )
        .ok();
    }

    /// Recognition worked, so whatever was wrong is over.
    fn clear(&mut self) {
        self.last = None;
    }
}

/// How a session turns audio into words.
///
/// The two are different pipelines rather than two settings on one. A batch
/// engine is handed whole sentences that the endpointer has already cut out of
/// the stream, and answers each one on its own. A live engine is handed the
/// stream itself and answers continuously, doing its own endpointing at the far
/// end. See ADR-004.
pub enum Recogniser {
    /// One request per utterance, decoded on a single worker thread.
    Batch(Box<dyn AsrEngine>),
    /// A socket per source, fed audio as it is captured.
    Live(audis_asr::LiveConfig),
}

/// What to start, gathered from settings by the caller.
pub struct SessionRequest {
    /// What will recognise speech, already built and ready.
    pub recogniser: Recogniser,
    pub diarize: Option<Diarizer>,
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
    /// Show the assistant panel, because the assistant is on.
    pub assistant_enabled: bool,
    /// Save each source's audio to the recordings folder as Opus.
    pub record: bool,
}

/// One utterance, tagged with where it came from.
struct SourcedUtterance {
    source: AudioSourceKind,
    utterance: audis_asr::Utterance,
}

/// One thing a live socket said, tagged with where and when.
struct SourcedWords {
    source: AudioSourceKind,
    /// Milliseconds of audio this source had sent when the words arrived.
    ///
    /// Audio time rather than wall time, so a paused session leaves no gap in
    /// the transcript and timestamps line up with the recording.
    at_ms: i64,
    event: LiveEvent,
}

/// The newest sentence-in-progress waiting for an interim decode.
type PartialSlot = Arc<Mutex<Option<SourcedUtterance>>>;

/// A started source: its capture, and every thread started for it.
type SourceHandles = (audis_audio::CaptureHandle, Vec<JoinHandle<()>>);

/// The half of the pipeline every source shares, built once per session.
enum Shared {
    /// The endpointer cuts sentences and one worker decodes them.
    Batch {
        utterances: SyncSender<SourcedUtterance>,
        partial: PartialSlot,
        endpoint_config: EndpointConfig,
        /// Whether the engine is cheap enough to re-decode a sentence mid-flow.
        want_partials: bool,
    },
    /// Each source gets its own socket, and all of them report here.
    Live {
        config: audis_asr::LiveConfig,
        words: SyncSender<SourcedWords>,
    },
}

/// The half of the pipeline that belongs to one source.
enum SourcePipeline {
    Batch {
        utterances: SyncSender<SourcedUtterance>,
        partial: PartialSlot,
        endpoint_config: EndpointConfig,
        want_partials: bool,
    },
    Live {
        input: LiveInput,
        output: LiveOutput,
        words: SyncSender<SourcedWords>,
    },
}

impl Shared {
    /// Build the half that belongs to `source`.
    ///
    /// A live session opens its socket here, before any device is touched, so a
    /// refused key fails the session with the reason rather than opening a
    /// microphone and then captioning nothing.
    fn for_source(&self) -> Result<SourcePipeline> {
        match self {
            Self::Batch {
                utterances,
                partial,
                endpoint_config,
                want_partials,
            } => Ok(SourcePipeline::Batch {
                utterances: utterances.clone(),
                partial: Arc::clone(partial),
                endpoint_config: *endpoint_config,
                want_partials: *want_partials,
            }),
            Self::Live { config, words } => {
                let (input, output) = audis_asr::live::connect(config).map_err(|error| {
                    // Logged as well as returned: without this line a failed
                    // session leaves nothing in the log at all, and the reason
                    // exists only in a window the user has already dismissed.
                    tracing::error!(%error, "the live speech socket could not be opened");
                    AudisError::Engine(Box::new(error.to_user_facing()))
                })?;
                Ok(SourcePipeline::Live {
                    input,
                    output,
                    words: words.clone(),
                })
            }
        }
    }
}

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
    pub fn start(&self, app: &AppHandle, request: SessionRequest) -> Result<SessionStatus> {
        let SessionRequest {
            recogniser,
            diarize,
            paths,
            mode,
            language,
            microphone_id,
            computer_audio_id,
            want_microphone,
            want_computer_audio,
            assistant_enabled,
            record,
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

        let writer = if mode.persists_transcript() {
            Some(crate::transcript_store::SessionWriter::create(
                &paths, id, mode, language,
            )?)
        } else {
            None
        };

        let stop = Arc::new(AtomicBool::new(false));
        let paused = Arc::new(AtomicBool::new(false));

        let mut captures = Vec::new();
        let mut workers = Vec::new();

        let (shared, recogniser_thread) = match recogniser {
            Recogniser::Batch(engine) => {
                // A local engine decodes for free, so it can afford live
                // previews of the sentence in progress. A batch cloud engine
                // bills every round-trip, so it gets short chunks and no
                // previews instead.
                let offline = engine.capabilities().offline;
                let endpoint_config = if offline {
                    EndpointConfig::default()
                } else {
                    EndpointConfig {
                        silence_ms: CLOUD_SILENCE_MS,
                        max_utterance_ms: CLOUD_MAX_UTTERANCE_MS,
                        ..EndpointConfig::default()
                    }
                };

                let partial: PartialSlot = Arc::new(Mutex::new(None));
                let (utterance_tx, utterance_rx) =
                    sync_channel::<SourcedUtterance>(UTTERANCE_QUEUE_DEPTH);

                let thread = spawn_recogniser(RecogniserSetup {
                    app: app.clone(),
                    session_id: id,
                    language,
                    engine,
                    diarize,
                    utterances: utterance_rx,
                    partial: Arc::clone(&partial),
                    stop: Arc::clone(&stop),
                    writer,
                });

                (
                    Shared::Batch {
                        utterances: utterance_tx,
                        partial,
                        endpoint_config,
                        want_partials: offline,
                    },
                    thread,
                )
            }

            Recogniser::Live(config) => {
                // Speaker separation reads the audio of a finished utterance,
                // and a live socket never hands one over: the words arrive on
                // their own. Saying so is better than quietly labelling every
                // line with the source it came from.
                if diarize.is_some() {
                    tracing::info!("speaker separation is off: the live engine has no utterances");
                    app.emit(
                        events::DIAGNOSTIC_WARNING,
                        DiagnosticWarning {
                            kind: "speakers.unavailable".to_owned(),
                            message: "Speaker separation is off for this session. The live \
                                      engine transcribes as people speak, which leaves no \
                                      audio to tell voices apart with."
                                .to_owned(),
                        },
                    )
                    .ok();
                }

                let (word_tx, word_rx) = sync_channel::<SourcedWords>(WORD_QUEUE_DEPTH);

                let thread = spawn_live_recogniser(LiveRecogniserSetup {
                    app: app.clone(),
                    session_id: id,
                    language,
                    words: word_rx,
                    stop: Arc::clone(&stop),
                    writer,
                });

                (
                    Shared::Live {
                        config,
                        words: word_tx,
                    },
                    thread,
                )
            }
        };

        workers.push(
            recogniser_thread.map_err(|error| AudisError::Configuration {
                detail: format!("the recognition thread could not start: {error}"),
            })?,
        );

        let mut started_microphone = false;
        let mut started_computer_audio = false;

        let record_path = |source: AudioSourceKind| {
            record.then(|| {
                paths
                    .recordings_dir()
                    .join(format!("session-{id}-{}.ogg", source_slug(source)))
            })
        };

        for (source, wanted, device_id, started) in [
            (
                AudioSourceKind::Microphone,
                want_microphone,
                microphone_id.as_deref(),
                &mut started_microphone,
            ),
            (
                AudioSourceKind::ComputerAudio,
                want_computer_audio,
                computer_audio_id.as_deref(),
                &mut started_computer_audio,
            ),
        ] {
            if !wanted {
                continue;
            }

            // A live socket that will not open is the whole session's problem,
            // not one source's: every source would fail the same way, and the
            // reason is something the user has to fix rather than retry.
            let pipeline = match shared.for_source() {
                Ok(pipeline) => pipeline,
                Err(error) => {
                    stop.store(true, Ordering::SeqCst);
                    // Before joining, not after: a prepare thread is parked on
                    // its frame channel, and only closing the device ends it.
                    captures.clear();
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
                    return Err(error);
                }
            };

            match start_source(
                app,
                source,
                device_id,
                pipeline,
                Arc::clone(&stop),
                Arc::clone(&paused),
                record_path(source),
            ) {
                Ok((capture, threads)) => {
                    captures.push(capture);
                    workers.extend(threads);
                    *started = true;
                }
                Err(error) => {
                    tracing::warn!(%error, ?source, "capture could not start");
                    emit_source_failed(app, source, &error);
                }
            }
        }

        drop(shared);

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

        crate::overlays::show(app, crate::overlays::Overlay::Captions);
        crate::overlays::show(app, crate::overlays::Overlay::Controller);
        if assistant_enabled {
            // The panel shows itself once it has an answer; just put it in place.
            crate::overlays::place(app, crate::overlays::Overlay::Assistant);
        }

        app.emit(events::SESSION_STATE, &status).ok();
        tracing::info!(%id, ?mode, "session started");

        Ok(status)
    }

    /// Pause or resume. Audio keeps flowing while paused and is discarded, so
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

        crate::overlays::hide_all(app);
        app.emit(events::SESSION_STATE, &status).ok();
        tracing::info!(%id, elapsed_ms = elapsed, "session stopped");

        Ok(status)
    }

    /// The current session, if one is running.
    pub fn status(&self) -> Option<SessionStatus> {
        self.lock().as_ref().map(Running::status)
    }

    /// A poisoned lock means a worker panicked while holding it. The session is
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

/// Open one capture and spawn the threads that carry its audio.
fn start_source(
    app: &AppHandle,
    source: AudioSourceKind,
    device_id: Option<&str>,
    pipeline: SourcePipeline,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    record_path: Option<std::path::PathBuf>,
) -> std::result::Result<SourceHandles, audis_audio::AudioError> {
    let (frame_tx, frame_rx) = sync_channel::<Vec<f32>>(FRAME_QUEUE_DEPTH);
    let dropped = Arc::new(AtomicU64::new(0));

    let (record_tx, record_rx) = if record_path.is_some() {
        let (tx, rx) = sync_channel::<Vec<f32>>(RECORD_QUEUE_DEPTH);
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };

    let sink_dropped = Arc::clone(&dropped);
    let sink_paused = Arc::clone(&paused);
    let sink: audis_audio::FrameSink = Arc::new(move |data: &[f32]| {
        if frame_tx.try_send(data.to_vec()).is_err() {
            sink_dropped.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(tx) = &record_tx
            && !sink_paused.load(Ordering::Relaxed)
        {
            let _ = tx.try_send(data.to_vec());
        }
    });

    let capture = match source {
        AudioSourceKind::Microphone => audis_audio::start_microphone_with_sink(device_id, sink)?,
        AudioSourceKind::ComputerAudio => {
            audis_audio::start_computer_audio_with_sink(device_id, sink)?
        }
    };

    let unstarted = |error: std::io::Error| audis_audio::AudioError::StreamStart {
        device: capture.device_name().to_owned(),
        detail: format!("the audio pipeline thread could not start: {error}"),
    };

    let mut threads = Vec::new();

    match pipeline {
        SourcePipeline::Batch {
            utterances,
            partial,
            endpoint_config,
            want_partials,
        } => threads.push(
            spawn_prepare(
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
                endpoint_config,
                want_partials,
            )
            .map_err(unstarted)?,
        ),

        SourcePipeline::Live {
            input,
            output,
            words,
        } => {
            // Both threads read this: one counts the audio it has sent, the
            // other stamps the words that come back with it.
            let clock = Arc::new(AtomicU64::new(0));

            threads.push(
                spawn_live_prepare(
                    app.clone(),
                    source,
                    capture.sample_rate(),
                    capture.channels(),
                    frame_rx,
                    input,
                    Arc::clone(&stop),
                    paused,
                    dropped,
                    Arc::clone(&clock),
                )
                .map_err(unstarted)?,
            );
            threads.push(spawn_live_pump(source, output, words, clock, stop).map_err(unstarted)?);
        }
    }

    if let (Some(rx), Some(path)) = (record_rx, record_path) {
        match crate::recorder::spawn(path, capture.sample_rate(), capture.channels(), rx) {
            Ok(handle) => threads.push(handle),
            Err(error) => tracing::warn!(%error, ?source, "recording could not start"),
        }
    }

    Ok((capture, threads))
}

/// A short, filesystem-safe name for a source, used in recording filenames.
fn source_slug(source: AudioSourceKind) -> &'static str {
    match source {
        AudioSourceKind::Microphone => "microphone",
        AudioSourceKind::ComputerAudio => "computer-audio",
    }
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
    endpoint_config: EndpointConfig,
    want_partials: bool,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("audis-prepare-{source:?}"))
        .spawn(move || {
            let mut resampler = Resampler::new(sample_rate);
            let mut endpointer = Endpointer::new(endpoint_config);
            let mut reported_drops = 0u64;
            let mut last_partial = std::time::Instant::now();

            while let Ok(block) = frames.recv() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                if paused.load(Ordering::SeqCst) {
                    continue;
                }

                let mono = downmix_to_mono(&block, channels);
                let ready = resampler.push(&mono);
                if ready.is_empty() {
                    continue;
                }

                let event = endpointer.push(&ready);

                if want_partials
                    && matches!(
                        event,
                        EndpointEvent::SpeechStarted | EndpointEvent::Speaking
                    )
                    && last_partial.elapsed() >= PARTIAL_INTERVAL
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

                if let EndpointEvent::Utterance(utterance) = event
                    && utterances
                        .try_send(SourcedUtterance { source, utterance })
                        .is_err()
                {
                    tracing::warn!(?source, "recognition is behind; dropped an utterance");
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

            if let Some(utterance) = endpointer.flush() {
                let _ = utterances.try_send(SourcedUtterance { source, utterance });
            }

            tracing::debug!(?source, "prepare thread finished");
        })
}

/// Downmix and resample one source's audio straight into its live socket.
///
/// The batch pipeline's endpointer has no part here: the socket is fed the
/// stream as it arrives and Gemini decides where the sentences are.
#[allow(clippy::too_many_arguments)]
fn spawn_live_prepare(
    app: AppHandle,
    source: AudioSourceKind,
    sample_rate: u32,
    channels: u16,
    frames: Receiver<Vec<f32>>,
    mut input: LiveInput,
    stop: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    dropped: Arc<AtomicU64>,
    clock: Arc<AtomicU64>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("audis-live-{source:?}"))
        .spawn(move || {
            let mut resampler = Resampler::new(sample_rate);
            let mut reported_drops = 0u64;

            while let Ok(block) = frames.recv() {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                // Paused audio is discarded rather than sent, so a pause costs
                // nothing and leaves no gap in the transcript's clock either.
                if paused.load(Ordering::SeqCst) {
                    continue;
                }

                let mono = downmix_to_mono(&block, channels);
                let ready = resampler.push(&mono);
                if ready.is_empty() {
                    continue;
                }

                clock.fetch_add(ready.len() as u64, Ordering::Relaxed);
                input.push(&ready);

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

            // Whatever is buffered is still part of the last sentence.
            input.flush();
            drop(input);

            tracing::debug!(?source, "live prepare thread finished");
        })
}

/// Carry one socket's words to the thread that publishes them.
///
/// A thread of its own rather than a poll on the prepare thread: a source that
/// has gone quiet stops delivering audio, and the last sentence must still come
/// through.
fn spawn_live_pump(
    source: AudioSourceKind,
    output: LiveOutput,
    words: SyncSender<SourcedWords>,
    clock: Arc<AtomicU64>,
    stop: Arc<AtomicBool>,
) -> std::io::Result<JoinHandle<()>> {
    std::thread::Builder::new()
        .name(format!("audis-live-words-{source:?}"))
        .spawn(move || {
            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                match output.recv_timeout(POLL) {
                    Ok(event) => {
                        let at_ms = samples_to_ms(clock.load(Ordering::Relaxed));
                        if words
                            .try_send(SourcedWords {
                                source,
                                at_ms,
                                event,
                            })
                            .is_err()
                        {
                            tracing::warn!(?source, "recognition is behind; dropped a caption");
                        }
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                    // The socket's own thread has ended, which it does once the
                    // prepare thread lets go of the audio end.
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            tracing::debug!(?source, "live words thread finished");
        })
}

/// Audio sent, as a position in the session.
fn samples_to_ms(samples: u64) -> i64 {
    (samples * 1000 / u64::from(audis_asr::TARGET_SAMPLE_RATE)) as i64
}

/// What the live recognise thread needs to run.
struct LiveRecogniserSetup {
    app: AppHandle,
    session_id: Uuid,
    language: Language,
    words: Receiver<SourcedWords>,
    stop: Arc<AtomicBool>,
    /// `None` for a mode that promises not to write to disk.
    writer: Option<crate::transcript_store::SessionWriter>,
}

/// Turn what the live sockets heard into transcript events.
///
/// One thread for every source, as in the batch pipeline, so the transcript
/// file has a single writer and the ordering of what reaches it is decided in
/// one place.
fn spawn_live_recogniser(setup: LiveRecogniserSetup) -> std::io::Result<JoinHandle<()>> {
    let LiveRecogniserSetup {
        app,
        session_id,
        language,
        words,
        stop,
        mut writer,
    } = setup;

    std::thread::Builder::new()
        .name("audis-recognise-live".to_owned())
        .spawn(move || {
            let engine_id = audis_asr::live::ENGINE_ID;
            let mut problems = ProblemReporter::default();
            // Where the sentence in progress began, per source.
            let mut opened: [Option<i64>; 2] = [None, None];

            for source in [AudioSourceKind::Microphone, AudioSourceKind::ComputerAudio] {
                emit_asr_status(&app, source, AsrState::Listening, engine_id, None);
            }

            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

                let heard = match words.recv_timeout(POLL) {
                    Ok(heard) => heard,
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                };

                let SourcedWords {
                    source,
                    at_ms,
                    event,
                } = heard;
                let began = &mut opened[usize::from(source == AudioSourceKind::ComputerAudio)];

                match event {
                    LiveEvent::Partial(text) => {
                        let start_ms = *began.get_or_insert(at_ms);
                        publish(
                            &app,
                            segment(
                                session_id, source, language, engine_id, start_ms, at_ms, text,
                                false,
                            ),
                            None,
                        );
                    }

                    LiveEvent::Final(text) => {
                        problems.clear();
                        let start_ms = began.take().unwrap_or(at_ms);
                        publish(
                            &app,
                            segment(
                                session_id, source, language, engine_id, start_ms, at_ms, text,
                                true,
                            ),
                            writer.as_mut(),
                        );
                    }

                    LiveEvent::Failed(error) => {
                        tracing::warn!(%error, ?source, "the live socket is failing");
                        problems.report(&app, &error);
                        emit_asr_status(
                            &app,
                            source,
                            AsrState::Listening,
                            engine_id,
                            Some(error.to_string()),
                        );
                    }
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
                emit_asr_status(&app, source, AsrState::Stopped, engine_id, None);
            }

            tracing::debug!("live recognise thread finished");
        })
}

/// One caption, from words a live socket sent.
#[allow(clippy::too_many_arguments)]
fn segment(
    session_id: Uuid,
    source: AudioSourceKind,
    language: Language,
    engine: &str,
    start_ms: i64,
    end_ms: i64,
    text: String,
    is_final: bool,
) -> TranscriptSegment {
    TranscriptSegment {
        id: Uuid::new_v4(),
        session_id,
        source,
        speaker: Some(source.default_label().to_owned()),
        start_ms,
        end_ms,
        text,
        language,
        // Gemini reports no per-sentence confidence over the socket.
        confidence: None,
        is_final,
        engine: engine.to_owned(),
    }
}

/// Save a segment when it is final, and publish it either way.
fn publish(
    app: &AppHandle,
    segment: TranscriptSegment,
    writer: Option<&mut crate::transcript_store::SessionWriter>,
) {
    if segment.is_empty_speech() {
        tracing::debug!(source = ?segment.source, "dropped a silence hallucination");
        return;
    }

    if !segment.is_final {
        app.emit(events::TRANSCRIPT_PARTIAL, &segment).ok();
        return;
    }

    tracing::debug!(
        source = ?segment.source,
        chars = segment.text.chars().count(),
        "final segment recognised"
    );

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
}

/// What the recognise thread needs to run.
struct RecogniserSetup {
    app: AppHandle,
    session_id: Uuid,
    language: Language,
    engine: Box<dyn AsrEngine>,
    diarize: Option<Diarizer>,
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
        mut diarize,
        utterances,
        partial,
        stop,
        mut writer,
    } = setup;

    std::thread::Builder::new()
        .name("audis-recognise".to_owned())
        .spawn(move || {
            let engine_id = engine.id().to_owned();
            let mut problems = ProblemReporter::default();

            for source in [AudioSourceKind::Microphone, AudioSourceKind::ComputerAudio] {
                emit_asr_status(&app, source, AsrState::Listening, &engine_id, None);
            }

            loop {
                if stop.load(Ordering::SeqCst) {
                    break;
                }

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
                            diarize.as_mut(),
                            writer.as_mut(),
                            &mut problems,
                        );
                        continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }

                // Only a local engine offers previews, and the prepare thread
                // already paces them; a cloud engine leaves this slot empty, so
                // this never fires for one.
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

                match utterances.recv_timeout(std::time::Duration::from_millis(40)) {
                    Ok(SourcedUtterance { source, utterance }) => decode_final(
                        &app,
                        engine.as_mut(),
                        &engine_id,
                        session_id,
                        language,
                        source,
                        &utterance,
                        diarize.as_mut(),
                        writer.as_mut(),
                        &mut problems,
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
#[allow(clippy::too_many_arguments)]
fn decode_final(
    app: &AppHandle,
    engine: &mut dyn AsrEngine,
    engine_id: &str,
    session_id: Uuid,
    language: Language,
    source: AudioSourceKind,
    utterance: &audis_asr::Utterance,
    diarize: Option<&mut Diarizer>,
    writer: Option<&mut crate::transcript_store::SessionWriter>,
    problems: &mut ProblemReporter,
) {
    emit_asr_status(app, source, AsrState::Recognising, engine_id, None);

    let result = match engine.transcribe(utterance, language) {
        Ok(result) => result,
        Err(error) => {
            tracing::warn!(%error, ?source, "an utterance could not be recognised");
            // The log is for us; this is what the user actually gets told.
            problems.report(app, &error);
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

    problems.clear();

    let mut segment = TranscriptSegment {
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

    if segment.is_empty_speech() {
        tracing::debug!(?source, "dropped a silence hallucination");
        tracing::trace!(text = %segment.text, "hallucination text");
        emit_asr_status(app, source, AsrState::Listening, engine_id, None);
        return;
    }

    if source == AudioSourceKind::ComputerAudio
        && let Some(diarizer) = diarize
        && let Some(assignment) = diarizer.identify(&utterance.samples)
    {
        let label = audis_asr::speaker_label(assignment.index);
        if assignment.is_new {
            app.emit(
                events::SPEAKER_UPDATE,
                SpeakerUpdate {
                    session_id,
                    source,
                    id: audis_asr::speaker_id(assignment.index),
                    label: label.clone(),
                    is_new: true,
                },
            )
            .ok();
        }
        segment.speaker = Some(label);
    }

    tracing::debug!(
        ?source,
        chars = segment.text.chars().count(),
        confidence = ?segment.confidence,
        "final segment recognised"
    );

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
fn decode_partial(
    app: &AppHandle,
    engine: &mut dyn AsrEngine,
    session_id: Uuid,
    language: Language,
    source: AudioSourceKind,
    utterance: &audis_asr::Utterance,
) {
    let Ok(result) = engine.transcribe(utterance, language) else {
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

    tracing::debug!(
        ?source,
        audio_ms = utterance.end_ms - utterance.start_ms,
        chars = segment.text.chars().count(),
        "interim caption"
    );
    tracing::trace!(text = %segment.text, "interim caption text");

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

    /// Live timestamps are counted in audio, not in wall time, so that a
    /// paused session leaves no gap and a segment lines up with the recording.
    #[test]
    fn live_timestamps_count_the_audio_that_was_sent() {
        assert_eq!(samples_to_ms(0), 0);
        assert_eq!(samples_to_ms(16_000), 1_000);
        assert_eq!(samples_to_ms(8_000), 500);
    }

    #[test]
    fn a_live_segment_spans_from_the_first_word_to_the_last() {
        let segment = segment(
            Uuid::new_v4(),
            AudioSourceKind::ComputerAudio,
            Language::Indonesian,
            audis_asr::live::ENGINE_ID,
            1_200,
            4_800,
            "selamat pagi".to_owned(),
            true,
        );

        assert_eq!(segment.start_ms, 1_200);
        assert_eq!(segment.end_ms, 4_800);
        assert!(segment.is_final);
        assert_eq!(segment.engine, "gemini-live");
        // The socket reports no per-sentence confidence, and inventing one
        // would put a number in the transcript that means nothing.
        assert_eq!(segment.confidence, None);
    }

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
    #[test]
    fn a_local_only_mode_does_not_report_the_assistant_as_running() {
        let session = running(SessionState::Listening);
        assert!(!session.status().assistant_enabled);

        let mut meeting = running(SessionState::Listening);
        meeting.mode = SessionMode::MeetingAssistant;
        assert!(meeting.status().assistant_enabled);
    }
}
