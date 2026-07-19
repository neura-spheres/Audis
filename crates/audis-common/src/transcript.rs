//! Transcript segments and the events that carry them.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ipc::AudioSourceKind;
use crate::language::Language;

/// One piece of recognised speech.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    /// Stable id for this segment.
    pub id: Uuid,
    /// The session it belongs to.
    pub session_id: Uuid,
    /// Which stream it came from. Microphone is always the local user.
    pub source: AudioSourceKind,
    /// Speaker label, once one is known.
    pub speaker: Option<String>,
    /// Start offset from the beginning of the session.
    pub start_ms: i64,
    /// End offset from the beginning of the session.
    pub end_ms: i64,
    /// The recognised words.
    pub text: String,
    /// Which language was recognised.
    pub language: Language,
    /// Engine confidence, when it reports one.
    pub confidence: Option<f32>,
    /// False while the engine may still revise this.
    pub is_final: bool,
    /// Which engine produced it.
    pub engine: String,
}

impl TranscriptSegment {
    /// How long this segment covers.
    pub fn duration_ms(&self) -> i64 {
        (self.end_ms - self.start_ms).max(0)
    }

    /// True when there are no actual words.
    pub fn is_empty_speech(&self) -> bool {
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return true;
        }
        (trimmed.starts_with('[') && trimmed.ends_with(']'))
            || (trimmed.starts_with('(') && trimmed.ends_with(')'))
            || (trimmed.starts_with('*') && trimmed.ends_with('*'))
    }
}

/// Rolling meeting intelligence, carried on `audis://meeting/update`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingUpdate {
    /// A short running summary of the session so far.
    pub summary: String,
    /// Decisions or conclusions reached.
    pub decisions: Vec<String>,
    /// Concrete action items, ideally with an owner.
    pub action_items: Vec<String>,
}

/// A correction to an already-written segment, carried on
/// `audis://transcript/revision` and appended to the transcript file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentRevision {
    /// The segment being corrected.
    pub id: Uuid,
    /// The corrected words.
    pub text: String,
    /// The corrected speaker label, if changed.
    pub speaker: Option<String>,
}

/// A speaker becoming known, carried on `audis://speaker/update`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerUpdate {
    /// The session this speaker belongs to.
    pub session_id: Uuid,
    /// Which stream the speaker was heard on.
    pub source: AudioSourceKind,
    /// Stable machine id, such as `person-1`.
    pub id: String,
    /// Provisional display label, such as `Person 1`.
    pub label: String,
    /// True the first time this speaker is heard.
    pub is_new: bool,
}

/// Health of a recognition stream, carried on `audis://asr/status`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AsrState {
    /// Loading a model or connecting.
    Starting,
    /// Running normally.
    Listening,
    /// Speech detected, decoding in progress.
    Recognising,
    /// Temporarily degraded but still trying.
    Reconnecting,
    /// Stopped.
    Stopped,
    /// Failed and not retrying.
    Failed,
}

/// Payload for `audis://asr/status`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsrStatus {
    /// Which stream.
    pub source: AudioSourceKind,
    /// What it is doing.
    pub state: AsrState,
    /// Engine name, for the UI.
    pub engine: String,
    /// Set when `state` is `Failed`.
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(text: &str) -> TranscriptSegment {
        TranscriptSegment {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            source: AudioSourceKind::Microphone,
            speaker: None,
            start_ms: 1000,
            end_ms: 2500,
            text: text.to_owned(),
            language: Language::English,
            confidence: None,
            is_final: true,
            engine: "whisper-local".to_owned(),
        }
    }

    #[test]
    fn duration_is_derived_and_never_negative() {
        assert_eq!(segment("hello").duration_ms(), 1500);

        let mut backwards = segment("hello");
        backwards.end_ms = 0;
        assert_eq!(backwards.duration_ms(), 0);
    }

    /// Whisper's non-speech annotations must never reach a caption.
    #[test]
    fn bracketed_annotations_are_not_speech() {
        for text in [
            "[BLANK_AUDIO]",
            "(music playing)",
            "*silence*",
            "   ",
            "",
            "[ Silence ]",
        ] {
            assert!(
                segment(text).is_empty_speech(),
                "{text:?} should not count as speech"
            );
        }
    }

    #[test]
    fn real_speech_is_kept_even_with_punctuation() {
        for text in [
            "Hello there.",
            "Selamat pagi, apa kabar?",
            "The cost is (roughly) ten dollars",
        ] {
            assert!(
                !segment(text).is_empty_speech(),
                "{text:?} is real speech and must be kept"
            );
        }
    }

    #[test]
    fn segments_serialise_as_camel_case() {
        let json = serde_json::to_value(segment("hi")).unwrap();
        assert!(json.get("sessionId").is_some());
        assert!(json.get("isFinal").is_some());
        assert!(json.get("startMs").is_some());
    }
}
