//! Transcript segments and the events that carry them.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ipc::AudioSourceKind;
use crate::language::Language;

/// One piece of recognised speech.
///
/// A segment is either interim (`is_final == false`), meaning the engine may
/// still change it, or final, meaning it will not. Only final segments are
/// persisted; storing every interim hypothesis would fill the database with
/// text the engine itself already discarded.
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
    ///
    /// Whisper emits bracketed annotations such as `[BLANK_AUDIO]` or
    /// `(music)` for non-speech. Those are not transcript and must never reach
    /// a caption.
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
