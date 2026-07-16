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

    #[test]
    fn serialises_as_camel_case() {
        let json = serde_json::to_string(&Language::Indonesian).unwrap();
        assert_eq!(json, "\"indonesian\"");
    }
}
