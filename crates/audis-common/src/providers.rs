//! AI provider identity and configuration.

use serde::{Deserialize, Serialize};

/// An AI provider Audis can talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderId {
    /// Generous free tier. The sensible default.
    Gemini,
    /// Very cheap, strong at reasoning.
    DeepSeek,
    /// Free tier, extremely fast inference.
    Groq,
    /// Strong quality, no free tier.
    Anthropic,
    /// Any OpenAI-compatible endpoint, including a local one such as Ollama.
    OpenAiCompatible,
}

/// What a provider is and how to reach it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    /// Stable identifier.
    pub id: ProviderId,
    /// Name shown to the user.
    pub name: String,
    /// One line on why you would pick this one.
    pub summary: String,
    /// Where to get a key. Shown as a link.
    pub console_url: String,
    /// True when the provider has a usable free tier.
    pub free_tier: bool,
    /// Default model, used until the user picks another.
    pub default_model: String,
    /// Models known to work. The UI offers these; a custom one can be typed.
    pub models: Vec<String>,
    /// True when the endpoint is user-supplied rather than fixed.
    pub needs_endpoint: bool,
    /// How this provider transcribes speech, or `None` if it cannot.
    pub speech: Option<SpeechSupport>,
}

/// The shape of a provider's speech-to-text API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeechApi {
    /// `POST /audio/transcriptions`, multipart, as OpenAI defined it.
    OpenAiTranscriptions,
    /// Gemini's `generateContent` with the audio inline.
    GeminiInline,
}

/// How a provider transcribes speech.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSupport {
    /// Which request shape to use.
    pub api: SpeechApi,
    /// Base URL, or `None` when the user supplies the endpoint.
    pub base_url: Option<String>,
    /// The model used unless the user picks another.
    pub default_model: String,
    /// Models known to work for speech.
    pub models: Vec<String>,
    /// One line on what this costs and how good it is, in plain words.
    pub summary: String,
}

/// The shape of a provider's chat API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatApi {
    /// `POST /chat/completions`, as OpenAI defined it.
    OpenAiChat,
    /// Gemini's `generateContent`.
    GeminiGenerate,
    /// Anthropic's `POST /messages`.
    AnthropicMessages,
}

/// How a provider holds a conversation, for the assistant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSupport {
    /// Which request shape to use.
    pub api: ChatApi,
    /// Base URL, or `None` when the user supplies the endpoint.
    pub base_url: Option<String>,
    /// The model used unless the user picks another.
    pub default_model: String,
    /// Models known to work for chat.
    pub models: Vec<String>,
}

impl ProviderId {
    /// Every provider, best default first.
    pub const ALL: [Self; 5] = [
        Self::Gemini,
        Self::Groq,
        Self::DeepSeek,
        Self::Anthropic,
        Self::OpenAiCompatible,
    ];

    /// The keystore entry name for this provider's key.
    pub fn credential_ref(self) -> String {
        format!("provider/{}/default", self.slug())
    }

    /// Short lowercase identifier, used in the credential reference.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Gemini => "gemini",
            Self::DeepSeek => "deepseek",
            Self::Groq => "groq",
            Self::Anthropic => "anthropic",
            Self::OpenAiCompatible => "openai-compatible",
        }
    }

    /// How this provider transcribes speech, or `None` if it cannot.
    pub fn speech(self) -> Option<SpeechSupport> {
        match self {
            Self::Groq => Some(SpeechSupport {
                api: SpeechApi::OpenAiTranscriptions,
                base_url: Some("https://api.groq.com/openai/v1".to_owned()),
                default_model: "whisper-large-v3-turbo".to_owned(),
                models: vec![
                    "whisper-large-v3-turbo".to_owned(),
                    "whisper-large-v3".to_owned(),
                ],
                summary: "Free tier, and fast enough to keep up with live speech. Runs \
                          whisper-large-v3, which is far better at Indonesian than anything \
                          Audis can run on your CPU."
                    .to_owned(),
            }),
            Self::Gemini => Some(SpeechSupport {
                api: SpeechApi::GeminiInline,
                base_url: Some("https://generativelanguage.googleapis.com/v1beta".to_owned()),
                default_model: "gemini-2.0-flash".to_owned(),
                models: vec![
                    "gemini-2.0-flash".to_owned(),
                    "gemini-2.0-flash-lite".to_owned(),
                ],
                summary: "Free tier. Good at Indonesian, though it is a general model asked to \
                          transcribe rather than a dedicated speech engine."
                    .to_owned(),
            }),
            Self::OpenAiCompatible => Some(SpeechSupport {
                api: SpeechApi::OpenAiTranscriptions,
                base_url: None,
                default_model: "whisper-1".to_owned(),
                models: vec![
                    "whisper-1".to_owned(),
                    "gpt-4o-transcribe".to_owned(),
                    "gpt-4o-mini-transcribe".to_owned(),
                ],
                summary: "Any endpoint that speaks OpenAI's transcription API. Use \
                          https://api.openai.com/v1 for OpenAI itself, which is paid and \
                          accurate, or point it at a whisper server on your own network."
                    .to_owned(),
            }),
            Self::DeepSeek | Self::Anthropic => None,
        }
    }

    /// True when this provider can transcribe speech.
    pub fn can_transcribe(self) -> bool {
        self.speech().is_some()
    }

    /// How this provider holds a conversation for the assistant.
    ///
    /// Every provider can chat, so this is never `None`.
    pub fn chat(self) -> ChatSupport {
        match self {
            Self::Gemini => ChatSupport {
                api: ChatApi::GeminiGenerate,
                base_url: Some("https://generativelanguage.googleapis.com/v1beta".to_owned()),
                default_model: "gemini-2.0-flash".to_owned(),
                models: vec!["gemini-2.0-flash".to_owned(), "gemini-2.5-flash".to_owned()],
            },
            Self::Groq => ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: Some("https://api.groq.com/openai/v1".to_owned()),
                default_model: "llama-3.3-70b-versatile".to_owned(),
                models: vec![
                    "llama-3.3-70b-versatile".to_owned(),
                    "llama-3.1-8b-instant".to_owned(),
                ],
            },
            Self::DeepSeek => ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: Some("https://api.deepseek.com".to_owned()),
                default_model: "deepseek-chat".to_owned(),
                models: vec!["deepseek-chat".to_owned(), "deepseek-reasoner".to_owned()],
            },
            Self::Anthropic => ChatSupport {
                api: ChatApi::AnthropicMessages,
                base_url: Some("https://api.anthropic.com/v1".to_owned()),
                default_model: "claude-3-5-haiku-latest".to_owned(),
                models: vec![
                    "claude-3-5-haiku-latest".to_owned(),
                    "claude-3-5-sonnet-latest".to_owned(),
                ],
            },
            Self::OpenAiCompatible => ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: None,
                default_model: "gpt-4o-mini".to_owned(),
                models: vec!["gpt-4o-mini".to_owned(), "gpt-4o".to_owned()],
            },
        }
    }

    /// Catalogue entry.
    pub fn info(self) -> ProviderInfo {
        let (name, summary, console_url, free_tier, default_model, models, needs_endpoint) =
            match self {
                Self::Gemini => (
                    "Google Gemini",
                    "Free tier that is generous enough for everyday use. Good at Indonesian.",
                    "https://aistudio.google.com/apikey",
                    true,
                    "gemini-2.0-flash",
                    vec!["gemini-2.0-flash", "gemini-2.0-flash-lite"],
                    false,
                ),
                Self::Groq => (
                    "Groq",
                    "Free tier and the fastest responses of any option here.",
                    "https://console.groq.com/keys",
                    true,
                    "llama-3.3-70b-versatile",
                    vec!["llama-3.3-70b-versatile", "llama-3.1-8b-instant"],
                    false,
                ),
                Self::DeepSeek => (
                    "DeepSeek",
                    "Very cheap, and strong at reasoning through a long meeting.",
                    "https://platform.deepseek.com/api_keys",
                    false,
                    "deepseek-chat",
                    vec!["deepseek-chat", "deepseek-reasoner"],
                    false,
                ),
                Self::Anthropic => (
                    "Anthropic",
                    "High quality summaries and answers. No free tier.",
                    "https://console.anthropic.com/settings/keys",
                    false,
                    "claude-haiku-4-5-20251001",
                    vec!["claude-haiku-4-5-20251001", "claude-sonnet-5"],
                    false,
                ),
                Self::OpenAiCompatible => (
                    "Custom (OpenAI-compatible)",
                    "Any OpenAI-compatible endpoint, including a local model server.",
                    "",
                    true,
                    "",
                    vec![],
                    true,
                ),
            };

        ProviderInfo {
            id: self,
            name: name.to_owned(),
            summary: summary.to_owned(),
            console_url: console_url.to_owned(),
            free_tier,
            default_model: default_model.to_owned(),
            models: models.into_iter().map(str::to_owned).collect(),
            needs_endpoint,
            speech: self.speech(),
        }
    }
}

/// A provider's configuration, as stored in settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    /// Which provider.
    pub id: ProviderId,
    /// Whether the user has enabled it.
    pub enabled: bool,
    /// Model to use.
    pub model: String,
    /// Endpoint, for OpenAI-compatible providers.
    pub endpoint: Option<String>,
    /// Keystore entry name. Never the key itself.
    pub credential_ref: String,
}

/// A provider's live state, for the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatus {
    /// Catalogue entry.
    pub info: ProviderInfo,
    /// Whether a key is saved. Never the key, and never a prefix of it.
    pub has_key: bool,
    /// Whether the user enabled it.
    pub enabled: bool,
    /// Currently selected model.
    pub model: String,
    /// Endpoint, for custom providers.
    pub endpoint: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_refs_are_unique_and_carry_no_secret() {
        let mut seen = std::collections::HashSet::new();
        for id in ProviderId::ALL {
            let reference = id.credential_ref();
            assert!(seen.insert(reference.clone()), "duplicate ref: {reference}");
            assert!(reference.starts_with("provider/"));
            assert!(reference.ends_with("/default"));
        }
    }

    #[test]
    fn every_provider_is_described() {
        for id in ProviderId::ALL {
            let info = id.info();
            assert!(!info.name.is_empty());
            assert!(!info.summary.is_empty());
            if !info.needs_endpoint {
                assert!(
                    !info.default_model.is_empty(),
                    "{} has no default model",
                    info.name
                );
                assert!(info.console_url.starts_with("https://"));
            }
        }
    }

    /// The product promise is that a free option exists.
    #[test]
    fn at_least_one_provider_has_a_free_tier() {
        assert!(ProviderId::ALL.iter().any(|id| id.info().free_tier));
    }

    /// The type that reaches the UI must be incapable of carrying a key.
    #[test]
    fn provider_status_has_no_field_that_could_hold_a_key() {
        let status = ProviderStatus {
            info: ProviderId::Gemini.info(),
            has_key: true,
            enabled: true,
            model: "gemini-2.0-flash".to_owned(),
            endpoint: None,
        };

        let json = serde_json::to_string(&status).expect("serialise");

        assert!(json.contains("hasKey"));
        for banned in ["apiKey", "api_key", "secret", "token", "credential"] {
            assert!(
                !json.contains(banned),
                "ProviderStatus leaked a {banned} field"
            );
        }
    }

    #[test]
    fn provider_config_stores_a_reference_not_a_value() {
        let config = ProviderConfig {
            id: ProviderId::Gemini,
            enabled: true,
            model: "gemini-2.0-flash".to_owned(),
            endpoint: None,
            credential_ref: ProviderId::Gemini.credential_ref(),
        };

        let json = serde_json::to_string(&config).expect("serialise");

        assert!(json.contains("provider/gemini/default"));
        assert!(!json.contains("apiKey"));
    }
}
