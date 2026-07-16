//! The recognition engine interface.

use audis_common::Language;

use crate::error::Result;
use crate::vad::Utterance;

/// What an engine can do. Declared per engine and verified, never assumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsrCapabilities {
    /// Runs without a network.
    pub offline: bool,
    /// Reports per-segment confidence.
    pub confidence: bool,
    /// Adds punctuation.
    pub punctuation: bool,
}

/// One recognised chunk.
#[derive(Debug, Clone, PartialEq)]
pub struct AsrResult {
    /// The words.
    pub text: String,
    /// Language recognised. Audis forces this, so it echoes the request.
    pub language: Language,
    /// Confidence, when the engine reports one.
    pub confidence: Option<f32>,
}

/// A speech recognition engine.
pub trait AsrEngine: Send {
    /// Stable identifier, for logs and the UI.
    fn id(&self) -> &'static str;

    /// What this engine supports.
    fn capabilities(&self) -> AsrCapabilities;

    /// Recognise one utterance.
    fn transcribe(&mut self, utterance: &Utterance, language: Language) -> Result<AsrResult>;
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;

    /// A deterministic engine for tests, so the pipeline can be exercised
    pub struct FakeEngine {
        pub reply: String,
    }

    impl AsrEngine for FakeEngine {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn capabilities(&self) -> AsrCapabilities {
            AsrCapabilities {
                offline: true,
                confidence: false,
                punctuation: false,
            }
        }

        fn transcribe(&mut self, _utterance: &Utterance, language: Language) -> Result<AsrResult> {
            Ok(AsrResult {
                text: self.reply.clone(),
                language,
                confidence: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::FakeEngine;
    use super::*;

    #[test]
    fn an_engine_echoes_the_requested_language_rather_than_guessing() {
        let mut engine = FakeEngine {
            reply: "selamat pagi".to_owned(),
        };
        let utterance = Utterance {
            samples: vec![0.0; 16_000],
            start_ms: 0,
            end_ms: 1000,
            truncated: false,
        };

        let result = engine
            .transcribe(&utterance, Language::Indonesian)
            .expect("fake engine");

        assert_eq!(result.language, Language::Indonesian);
        assert_eq!(result.text, "selamat pagi");
    }
}
