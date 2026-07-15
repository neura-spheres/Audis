//! The audio test: run both captures and stream their levels to the UI.
//!
//! Level events are emitted on a timer at [`LEVEL_HZ`], never from the audio
//! callback. The callback only stores numbers in atomics; this task reads them.
//! Emitting per audio block would flood the WebView with thousands of events a
//! second and stall the very thread that has to draw the meter.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use audis_audio::{AudioLevel, CaptureHandle};
use audis_common::{AudioSourceKind, ipc::events};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

/// Meter updates per second. Fast enough to look live, slow enough to be free.
const LEVEL_HZ: u64 = 25;

/// Payload for `audis://audio/level`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevelEvent {
    /// Which stream this reading came from.
    pub source: AudioSourceKind,
    /// Loudest sample in the last block, 0.0 to 1.0.
    pub peak: f32,
    /// Perceived loudness, 0.0 to 1.0. This is what the meter bar shows.
    pub rms: f32,
    /// True when the signal hit full scale and was probably clipped.
    pub clipping: bool,
    /// How long this source has been silent.
    pub silence_duration_ms: u64,
}

impl AudioLevelEvent {
    fn new(source: AudioSourceKind, level: AudioLevel) -> Self {
        Self {
            source,
            peak: level.peak,
            rms: level.rms,
            clipping: level.clipping,
            silence_duration_ms: level.silence_duration_ms,
        }
    }
}

/// What the UI shows once a test starts.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioTestStatus {
    /// Microphone stream, if it opened.
    pub microphone: Option<StreamStatus>,
    /// Loopback stream, if it opened.
    pub computer_audio: Option<StreamStatus>,
    /// Why a stream failed, when one did. One source failing does not stop the
    /// other: a missing microphone should not hide computer audio.
    pub microphone_error: Option<audis_common::UserFacingError>,
    /// Why loopback failed, when it did.
    pub computer_audio_error: Option<audis_common::UserFacingError>,
}

/// Details of one open stream.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamStatus {
    /// The device actually opened.
    pub device_name: String,
    /// Its real sample rate.
    pub sample_rate: u32,
    /// Its channel count.
    pub channels: u16,
}

impl StreamStatus {
    fn of(handle: &CaptureHandle) -> Self {
        Self {
            device_name: handle.device_name().to_owned(),
            sample_rate: handle.sample_rate(),
            channels: handle.channels(),
        }
    }
}

/// Holds the running test.
#[derive(Default)]
pub struct AudioTestState {
    running: Mutex<Option<RunningTest>>,
    /// Tells the emitter task to stop. Separate from the mutex so the task
    /// never has to take a lock the UI thread might hold.
    stop: std::sync::Arc<AtomicBool>,
}

struct RunningTest {
    microphone: Option<CaptureHandle>,
    computer_audio: Option<CaptureHandle>,
}

impl AudioTestState {
    /// Start both captures and begin emitting levels.
    ///
    /// Restarting while a test is running is fine: the previous one is stopped
    /// first, which is what a device picker change does.
    pub fn start(
        &self,
        app: &AppHandle,
        microphone_id: Option<String>,
        computer_audio_id: Option<String>,
    ) -> AudioTestStatus {
        self.stop_inner();

        let microphone = audis_audio::start_microphone(microphone_id.as_deref());
        let computer_audio = audis_audio::start_computer_audio(computer_audio_id.as_deref());

        let status = AudioTestStatus {
            microphone: microphone.as_ref().ok().map(StreamStatus::of),
            computer_audio: computer_audio.as_ref().ok().map(StreamStatus::of),
            microphone_error: microphone
                .as_ref()
                .err()
                .map(|error| error.to_user_facing()),
            computer_audio_error: computer_audio
                .as_ref()
                .err()
                .map(|error| error.to_user_facing()),
        };

        let running = RunningTest {
            microphone: microphone.ok(),
            computer_audio: computer_audio.ok(),
        };

        let stop = std::sync::Arc::clone(&self.stop);
        stop.store(false, Ordering::Relaxed);

        {
            let mut guard = self
                .running
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = Some(running);
        }

        spawn_level_emitter(app.clone(), stop);

        status
    }

    /// Stop the test and release both devices.
    pub fn stop(&self) {
        self.stop_inner();
    }

    fn stop_inner(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let mut guard = self
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Dropping the handles closes the streams and frees the devices, so
        // another app can use the microphone again.
        *guard = None;
    }

    /// Current levels for whichever streams are open.
    fn levels(&self) -> Vec<AudioLevelEvent> {
        let guard = self
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let Some(running) = guard.as_ref() else {
            return Vec::new();
        };

        let mut levels = Vec::with_capacity(2);
        if let Some(handle) = &running.microphone {
            levels.push(AudioLevelEvent::new(
                AudioSourceKind::Microphone,
                handle.level(),
            ));
        }
        if let Some(handle) = &running.computer_audio {
            levels.push(AudioLevelEvent::new(
                AudioSourceKind::ComputerAudio,
                handle.level(),
            ));
        }
        levels
    }
}

/// Emit level events on a timer until stopped.
fn spawn_level_emitter(app: AppHandle, stop: std::sync::Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        let period = std::time::Duration::from_millis(1000 / LEVEL_HZ);

        loop {
            tokio::time::sleep(period).await;

            if stop.load(Ordering::Relaxed) {
                break;
            }

            let state = app.state::<AudioTestState>();
            let levels = state.levels();
            if levels.is_empty() {
                break;
            }

            for level in levels {
                if let Err(error) = app.emit(events::AUDIO_LEVEL, &level) {
                    tracing::warn!(%error, "could not emit an audio level");
                    return;
                }
            }
        }

        tracing::debug!("level emitter stopped");
    });
}
