//! Watches for audio devices being added, removed, or made default.

use std::time::Duration;

use audis_common::events;
use tauri::{AppHandle, Emitter};

const POLL: Duration = Duration::from_secs(3);

/// Poll the audio devices and emit `AUDIO_DEVICE_CHANGE` whenever the set of
/// endpoints or the default choice changes.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("audis-device-watch".to_owned())
        .spawn(move || {
            let mut previous: Option<String> = None;
            loop {
                std::thread::sleep(POLL);
                let Ok(devices) = audis_audio::enumerate() else {
                    continue;
                };

                let fingerprint = fingerprint(&devices);
                if previous.as_deref() == Some(fingerprint.as_str()) {
                    continue;
                }

                let first = previous.is_none();
                previous = Some(fingerprint);
                if !first {
                    tracing::info!("audio devices changed");
                    app.emit(events::AUDIO_DEVICE_CHANGE, &devices).ok();
                }
            }
        })
        .ok();
}

fn fingerprint(devices: &audis_audio::AudioDevices) -> String {
    let mut parts = Vec::new();
    for device in devices.inputs.iter().chain(devices.outputs.iter()) {
        parts.push(format!("{}:{}", device.id, device.is_default));
    }
    parts.sort();
    parts.join("|")
}
