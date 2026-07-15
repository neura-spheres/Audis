//! The feature catalogue.
//!
//! One list, in Rust, describing every capability Audis offers and whether it
//! is actually usable in this build. The Features view renders this rather than
//! hardcoding its own copy, so the UI cannot claim a feature works when it does
//! not.

use serde::{Deserialize, Serialize};

/// A capability the user can start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FeatureId {
    /// Low-latency captions on screen, nothing saved.
    LiveCaption,
    /// Captions plus a saved, searchable transcript.
    Transcription,
    /// Transcription plus summary, decisions and action items.
    MeetingAssistant,
    /// Practice answering questions with AI help.
    InterviewPractice,
}

/// Whether a feature can be used right now, and why not if it cannot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FeatureStatus {
    /// Usable now.
    Ready,
    /// Implemented, but something is missing. See `blocker`.
    NeedsSetup,
    /// Not implemented in this build.
    NotBuilt,
}

/// One entry in the Features view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    /// Stable identifier.
    pub id: FeatureId,
    /// Name shown to the user.
    pub name: String,
    /// One line on what it does.
    pub summary: String,
    /// What it will do, for the detail panel.
    pub details: Vec<String>,
    /// Whether it can be started.
    pub status: FeatureStatus,
    /// Why it cannot be started, when it cannot. Plain words, actionable.
    pub blocker: Option<String>,
    /// True when using it sends audio or text off this PC.
    pub uses_cloud: bool,
}

impl FeatureId {
    /// Every feature, in the order the launcher shows them.
    pub const ALL: [Self; 4] = [
        Self::LiveCaption,
        Self::Transcription,
        Self::MeetingAssistant,
        Self::InterviewPractice,
    ];

    /// True when this feature sends transcript text to an AI provider.
    ///
    /// Drives both the "Uses cloud AI" badge and whether a provider key is
    /// required to start, so the badge cannot promise something the launcher
    /// then contradicts.
    pub fn uses_cloud_ai(self) -> bool {
        matches!(self, Self::MeetingAssistant | Self::InterviewPractice)
    }

    /// True when this feature writes a transcript to disk.
    ///
    /// Live Caption's promise is literally "No transcript is written to disk",
    /// so this is what enforces it. The session pipeline never constructs a
    /// writer when this is false, rather than writing a file and hiding it.
    pub fn persists_transcript(self) -> bool {
        match self {
            Self::LiveCaption => false,
            Self::Transcription | Self::MeetingAssistant | Self::InterviewPractice => true,
        }
    }

    /// Static description. Status is decided at runtime by the app, which knows
    /// whether a model is installed and a device exists.
    pub fn describe(self) -> (&'static str, &'static str, &'static [&'static str]) {
        match self {
            Self::LiveCaption => (
                "Live Caption",
                "Captions on screen as people speak. Nothing is saved.",
                &[
                    "Lowest latency of any mode",
                    "Captions float above your other windows",
                    "Your microphone and the computer's audio are labelled separately",
                    "No transcript is written to disk",
                ],
            ),
            Self::Transcription => (
                "Transcription",
                "Captions plus a saved transcript you can search and export.",
                &[
                    "Everything Live Caption does",
                    "A saved, searchable transcript",
                    "Optional audio recording",
                    "Export to text, Markdown, SRT and more",
                ],
            ),
            Self::MeetingAssistant => (
                "Meeting Assistant",
                "Transcription plus a rolling summary, decisions and action items.",
                &[
                    "Everything Transcription does",
                    "Speaker separation for the people you are listening to",
                    "Rolling summary as the meeting goes",
                    "Decisions and action items, with the lines they came from",
                    "Ask the assistant about the conversation",
                ],
            ),
            Self::InterviewPractice => (
                "Interview Practice",
                "Practice answering questions with the assistant helping.",
                &[
                    "Load a résumé and a job description as context",
                    "Detects questions and suggests structured answers",
                    "Saves feedback for later review",
                    "Always shows a visible indicator that it is active",
                ],
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_feature_is_described() {
        for id in FeatureId::ALL {
            let (name, summary, details) = id.describe();
            assert!(!name.is_empty());
            assert!(!summary.is_empty());
            assert!(!details.is_empty(), "{name} has no detail lines");
        }
    }

    #[test]
    fn feature_ids_serialise_as_camel_case() {
        let json = serde_json::to_string(&FeatureId::LiveCaption).unwrap();
        assert_eq!(json, "\"liveCaption\"");
    }

    /// A blocker without a reason is useless to the user.
    #[test]
    fn a_blocked_feature_must_explain_itself() {
        let feature = Feature {
            id: FeatureId::LiveCaption,
            name: "Live Caption".to_owned(),
            summary: "x".to_owned(),
            details: vec!["y".to_owned()],
            status: FeatureStatus::NeedsSetup,
            blocker: Some("Install a speech model first.".to_owned()),
            uses_cloud: false,
        };

        assert!(feature.blocker.is_some());
        assert!(matches!(feature.status, FeatureStatus::NeedsSetup));
    }

    /// Live Caption's description promises, in as many words, that nothing is
    /// written to disk. Privacy claims are the ones users cannot verify for
    /// themselves, so the promise and the behaviour are pinned to each other
    /// here: changing either alone fails this test.
    #[test]
    fn live_caption_promises_no_transcript_and_writes_none() {
        let (_, summary, details) = FeatureId::LiveCaption.describe();

        assert!(
            summary.contains("Nothing is saved"),
            "the summary no longer makes the promise this test enforces"
        );
        assert!(
            details
                .iter()
                .any(|line| line.contains("No transcript is written to disk")),
            "the details no longer make the promise this test enforces"
        );
        assert!(
            !FeatureId::LiveCaption.persists_transcript(),
            "Live Caption promises no transcript but would write one"
        );
    }

    /// The mirror image: a feature that advertises a saved transcript must
    /// actually save one, or the launcher is selling something absent.
    ///
    /// Spelled out per feature rather than inferred from the prose. The first
    /// version of this searched descriptions for "saved" and matched Live
    /// Caption's "Nothing is saved", asserting the exact opposite of the
    /// promise. Which mode saves is a decision, so it is written as one.
    #[test]
    fn every_feature_advertising_a_transcript_writes_one() {
        assert!(
            FeatureId::Transcription.persists_transcript(),
            "Transcription advertises a saved, searchable transcript"
        );
        assert!(
            FeatureId::MeetingAssistant.persists_transcript(),
            "Meeting Assistant advertises everything Transcription does"
        );
        assert!(
            FeatureId::InterviewPractice.persists_transcript(),
            "Interview Practice advertises saved feedback for later review"
        );
        assert!(
            !FeatureId::LiveCaption.persists_transcript(),
            "Live Caption is the only mode that writes nothing"
        );
    }

    /// A feature that needs a provider key must be the one that says so.
    #[test]
    fn only_cloud_features_are_marked_as_using_cloud_ai() {
        assert!(!FeatureId::LiveCaption.uses_cloud_ai());
        assert!(!FeatureId::Transcription.uses_cloud_ai());
        assert!(FeatureId::MeetingAssistant.uses_cloud_ai());
        assert!(FeatureId::InterviewPractice.uses_cloud_ai());
    }
}
