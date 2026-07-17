//! Typed IPC contracts shared by the Rust core and the React frontend.

use serde::{Deserialize, Serialize};

use crate::identity::EVENT_PREFIX;

/// Event channel names emitted by the Rust core.
pub mod events {
    /// Session lifecycle transitions.
    pub const SESSION_STATE: &str = "audis://session/state";

    /// Throttled level-meter updates. Never one event per audio frame.
    pub const AUDIO_LEVEL: &str = "audis://audio/level";

    /// Device added, removed, or default changed.
    pub const AUDIO_DEVICE_CHANGE: &str = "audis://audio/device-change";

    /// Interim ASR hypothesis. May change; never stored as final.
    pub const TRANSCRIPT_PARTIAL: &str = "audis://transcript/partial";

    /// Finalised ASR result. This is what gets persisted.
    pub const TRANSCRIPT_FINAL: &str = "audis://transcript/final";

    /// A correction to an already-finalised segment.
    pub const TRANSCRIPT_REVISION: &str = "audis://transcript/revision";

    /// ASR engine health: connecting, streaming, reconnecting, degraded.
    pub const ASR_STATUS: &str = "audis://asr/status";

    /// Speaker label assigned, renamed, merged or split.
    pub const SPEAKER_UPDATE: &str = "audis://speaker/update";

    /// AI assistant lifecycle.
    pub const ASSISTANT_STATUS: &str = "audis://assistant/status";

    /// Streamed AI assistant output.
    pub const ASSISTANT_RESPONSE: &str = "audis://assistant/response";

    /// Rolling meeting intelligence changed.
    pub const MEETING_UPDATE: &str = "audis://meeting/update";

    /// Update check, download and install progress.
    pub const UPDATE_STATUS: &str = "audis://update/status";

    /// How far a download of a new version has got.
    pub const UPDATE_PROGRESS: &str = "audis://update/progress";

    /// Non-fatal warning worth surfacing quietly, such as dropped frames.
    pub const DIAGNOSTIC_WARNING: &str = "audis://diagnostic/warning";

    /// Progress while a speech model downloads.
    pub const MODEL_PROGRESS: &str = "audis://model/progress";

    /// Settings were saved. Carries the whole `Settings`.
    pub const SETTINGS_CHANGED: &str = "audis://settings/changed";

    /// Every event this build can emit.
    pub const ALL: &[&str] = &[
        SESSION_STATE,
        AUDIO_LEVEL,
        AUDIO_DEVICE_CHANGE,
        TRANSCRIPT_PARTIAL,
        TRANSCRIPT_FINAL,
        TRANSCRIPT_REVISION,
        ASR_STATUS,
        SPEAKER_UPDATE,
        ASSISTANT_STATUS,
        ASSISTANT_RESPONSE,
        MEETING_UPDATE,
        UPDATE_STATUS,
        UPDATE_PROGRESS,
        DIAGNOSTIC_WARNING,
        MODEL_PROGRESS,
        SETTINGS_CHANGED,
    ];
}

/// Which audio source a frame, segment or level reading came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioSourceKind {
    /// Local microphone capture.
    Microphone,
    /// WASAPI loopback capture of system playback.
    ComputerAudio,
}

impl AudioSourceKind {
    /// The label shown before any diarization has run.
    pub fn default_label(self) -> &'static str {
        match self {
            Self::Microphone => "You",
            Self::ComputerAudio => "Computer Audio",
        }
    }
}

/// Build and identity information for the About page and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    /// Display name.
    pub app_name: String,
    /// Company and product family.
    pub company: String,
    /// Publisher shown in Add/Remove Programs.
    pub publisher: String,
    /// Product tagline.
    pub tagline: String,
    /// Semantic version of this build.
    pub version: String,
    /// OS bundle identifier.
    pub bundle_id: String,
    /// Absolute path to the data root.
    pub data_dir: String,
}

/// Payload for [`events::DIAGNOSTIC_WARNING`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticWarning {
    /// Machine-readable kind, for example `audio.frames_dropped`.
    pub kind: String,
    /// Short message. Must not contain transcript text.
    pub message: String,
}

/// True when `name` is a well-formed Audis event channel.
pub fn is_audis_event(name: &str) -> bool {
    name.starts_with(EVENT_PREFIX)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn every_event_uses_the_audis_prefix() {
        for name in events::ALL {
            assert!(is_audis_event(name), "{name} is missing the prefix");
        }
    }

    #[test]
    fn event_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for name in events::ALL {
            assert!(seen.insert(*name), "duplicate channel: {name}");
        }
    }

    #[test]
    fn microphone_and_computer_audio_have_distinct_default_labels() {
        assert_eq!(AudioSourceKind::Microphone.default_label(), "You");
        assert_eq!(
            AudioSourceKind::ComputerAudio.default_label(),
            "Computer Audio"
        );
    }

    #[test]
    fn audio_source_kind_serialises_as_camel_case() {
        let json = serde_json::to_string(&AudioSourceKind::ComputerAudio).unwrap();
        assert_eq!(json, "\"computerAudio\"");
    }
}
