//! Supported languages.

use serde::{Deserialize, Serialize};

/// A language Audis can recognise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Language {
    /// Bahasa Indonesia.
    Indonesian,
    /// English.
    #[default]
    English,
}

impl Language {
    /// ISO 639-1 code, which is what recognition engines expect.
    pub fn code(self) -> &'static str {
        match self {
            Self::Indonesian => "id",
            Self::English => "en",
        }
    }

    /// BCP-47 tag with a region, which the streaming engines take as a hint.
    pub fn bcp47(self) -> &'static str {
        match self {
            Self::Indonesian => "id-ID",
            Self::English => "en-US",
        }
    }

    /// The language's name in English, for instructions to a language model.
    pub fn english_name(self) -> &'static str {
        match self {
            Self::Indonesian => "Indonesian",
            Self::English => "English",
        }
    }

    /// A sentence in this language that primes a Whisper-family model to keep
    /// writing in it.
    ///
    /// Whisper reads its prompt as the transcript that came before, so a
    /// sentence of plain, correctly spelled text in the target language pulls
    /// every following word towards that language's vocabulary and spelling.
    /// That is a bias inside the model, not a filter after it: a borrowed word
    /// comes out the way this language writes it rather than being dropped.
    pub fn priming_prompt(self) -> &'static str {
        match self {
            Self::Indonesian => {
                "Berikut ini transkrip percakapan dalam bahasa Indonesia, \
                 ditulis dengan ejaan baku dan tanda baca yang benar."
            }
            Self::English => {
                "The following is a transcript of a conversation in English, \
                 written with standard spelling and punctuation."
            }
        }
    }

    /// Name in the language itself, as a picker should show it.
    pub fn endonym(self) -> &'static str {
        match self {
            Self::Indonesian => "Bahasa Indonesia",
            Self::English => "English",
        }
    }

    /// Every supported language.
    pub const ALL: [Self; 2] = [Self::English, Self::Indonesian];

    /// Parse an ISO 639-1 code.
    pub fn from_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "id" | "ind" => Some(Self::Indonesian),
            "en" | "eng" => Some(Self::English),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for language in Language::ALL {
            assert_eq!(Language::from_code(language.code()), Some(language));
        }
    }

    #[test]
    fn unsupported_languages_are_rejected_rather_than_guessed() {
        for code in ["fr", "de", "ja", "zh", ""] {
            assert_eq!(
                Language::from_code(code),
                None,
                "{code} must not be accepted"
            );
        }
    }

    #[test]
    fn codes_are_what_engines_expect() {
        assert_eq!(Language::Indonesian.code(), "id");
        assert_eq!(Language::English.code(), "en");
    }

    /// Streaming engines want a region as well as a language.
    #[test]
    fn bcp47_tags_carry_a_region() {
        assert_eq!(Language::Indonesian.bcp47(), "id-ID");
        assert_eq!(Language::English.bcp47(), "en-US");
    }

    /// The priming text must be in the language it primes, or it would pull
    /// the model the wrong way.
    #[test]
    fn priming_prompts_are_written_in_their_own_language() {
        assert!(
            Language::Indonesian
                .priming_prompt()
                .contains("bahasa Indonesia")
        );
        assert!(Language::English.priming_prompt().contains("in English"));
        for language in Language::ALL {
            assert!(!language.priming_prompt().contains("  "), "{language:?}");
        }
    }

    #[test]
    fn serialises_as_camel_case() {
        let json = serde_json::to_string(&Language::Indonesian).unwrap();
        assert_eq!(json, "\"indonesian\"");
    }
}
