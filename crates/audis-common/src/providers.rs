//! AI provider identity and configuration.

use serde::{Deserialize, Serialize};

/// Gemini's streaming speech model: audio in over a socket, words back as they
/// are spoken. The default, and the reason to pick Gemini for speech at all.
pub const GEMINI_LIVE_MODEL: &str = "gemini-3.8-live";

/// Gemini's fallback speech model: one request per finished sentence.
///
/// Google's current cheap stable model. Kept for anyone who would rather spend a
/// request per sentence than hold a socket open.
pub const GEMINI_BATCH_MODEL: &str = "gemini-3.5-flash-lite";

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
    /// A dedicated speech engine. Transcribes only; it does not converse.
    Deepgram,
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
    /// How this provider holds a conversation, or `None` if it cannot.
    pub chat: Option<ChatSupport>,
}

/// The shape of a provider's speech-to-text API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeechApi {
    /// `POST /audio/transcriptions`, multipart, as OpenAI defined it.
    OpenAiTranscriptions,
    /// Gemini's `generateContent` with the audio inline.
    GeminiInline,
    /// Deepgram's `POST /listen`, with the audio as the raw body.
    DeepgramListen,
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
    pub const ALL: [Self; 6] = [
        Self::Gemini,
        Self::Groq,
        Self::Deepgram,
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
            Self::Deepgram => "deepgram",
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
                default_model: GEMINI_LIVE_MODEL.to_owned(),
                models: vec![GEMINI_LIVE_MODEL.to_owned(), GEMINI_BATCH_MODEL.to_owned()],
                summary: "Free tier. The Live model transcribes as you speak rather than a \
                          sentence at a time, so captions keep up with the room. Good at \
                          Indonesian."
                    .to_owned(),
            }),
            Self::OpenAiCompatible => Some(SpeechSupport {
                api: SpeechApi::OpenAiTranscriptions,
                base_url: None,
                // whisper-1 stays the default: it is the one name a local
                // whisper server also answers to.
                default_model: "whisper-1".to_owned(),
                models: vec![
                    "gpt-transcribe".to_owned(),
                    "gpt-4o-transcribe".to_owned(),
                    "gpt-4o-mini-transcribe".to_owned(),
                    "whisper-1".to_owned(),
                ],
                summary: "Any endpoint that speaks OpenAI's transcription API. Use \
                          https://api.openai.com/v1 for OpenAI itself, which is paid and \
                          accurate, or point it at a whisper server on your own network."
                    .to_owned(),
            }),
            Self::Deepgram => Some(SpeechSupport {
                api: SpeechApi::DeepgramListen,
                base_url: Some("https://api.deepgram.com/v1".to_owned()),
                default_model: "nova-3".to_owned(),
                models: vec!["nova-3".to_owned(), "nova-2".to_owned()],
                summary: "A dedicated speech engine rather than a general model asked to \n                          transcribe, so it will not wander off and answer the \n                          audio. Fast, accurate, and good at both Indonesian and \n                          English."
                    .to_owned(),
            }),
            Self::DeepSeek | Self::Anthropic => None,
        }
    }

    /// True when this provider can transcribe speech.
    pub fn can_transcribe(self) -> bool {
        self.speech().is_some()
    }

    /// True when `model` recognises speech over a live streaming socket rather
    /// than one request per finished utterance.
    ///
    /// The two are different pipelines, not different settings: a streaming
    /// model is fed audio continuously and answers with words as it hears them,
    /// where the request-per-utterance models need speech cut into sentences
    /// first. Which one a session runs is decided by the model the user picked,
    /// so this is the one place that reads that out of a name.
    pub fn streams_speech(self, model: &str) -> bool {
        // Google names every bidirectional model `-live-`, and only those
        // models accept a socket. Matching the name means a new generation
        // works the day it appears rather than the day this list is edited.
        matches!(self, Self::Gemini) && model.to_lowercase().contains("live")
    }

    /// True when this provider can hold a conversation for the assistant.
    pub fn can_chat(self) -> bool {
        self.chat().is_some()
    }

    /// How this provider holds a conversation for the assistant, or `None` when
    /// it cannot.
    ///
    /// A dedicated speech engine transcribes and nothing else; offering it to
    /// the assistant would only produce a request it has no endpoint for.
    pub fn chat(self) -> Option<ChatSupport> {
        match self {
            Self::Gemini => Some(ChatSupport {
                api: ChatApi::GeminiGenerate,
                base_url: Some("https://generativelanguage.googleapis.com/v1beta".to_owned()),
                default_model: "gemini-3.8-flash".to_owned(),
                models: vec![
                    "gemini-3.8-flash".to_owned(),
                    "gemini-3.5-flash-lite".to_owned(),
                    "gemini-4-argon".to_owned(),
                    "gemini-3.1-pro-preview".to_owned(),
                ],
            }),
            Self::Groq => Some(ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: Some("https://api.groq.com/openai/v1".to_owned()),
                default_model: "openai/gpt-oss-120b".to_owned(),
                models: vec![
                    "openai/gpt-oss-120b".to_owned(),
                    "openai/gpt-oss-20b".to_owned(),
                    "qwen/qwen3.8-27b".to_owned(),
                    "llama-3.3-70b-versatile".to_owned(),
                    "llama-3.1-8b-instant".to_owned(),
                ],
            }),
            Self::DeepSeek => Some(ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: Some("https://api.deepseek.com".to_owned()),
                default_model: "deepseek-flash".to_owned(),
                models: vec!["deepseek-flash".to_owned(), "deepseek-v4-pro".to_owned()],
            }),
            Self::Anthropic => Some(ChatSupport {
                api: ChatApi::AnthropicMessages,
                base_url: Some("https://api.anthropic.com/v1".to_owned()),
                default_model: "claude-haiku-5-5".to_owned(),
                models: vec![
                    "claude-haiku-5-5".to_owned(),
                    "claude-sonnet-5-5".to_owned(),
                    "claude-opus-5-5".to_owned(),
                    "claude-fable-5-1".to_owned(),
                ],
            }),
            Self::OpenAiCompatible => Some(ChatSupport {
                api: ChatApi::OpenAiChat,
                base_url: None,
                default_model: "gpt-6-luna".to_owned(),
                models: vec![
                    "gpt-6-luna".to_owned(),
                    "gpt-6.1-sol".to_owned(),
                    "gpt-6-astra".to_owned(),
                ],
            }),
            // Deepgram transcribes and nothing else: it has no chat endpoint.
            Self::Deepgram => None,
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
                    "gemini-3.8-flash",
                    vec![
                        "gemini-3.8-flash",
                        "gemini-3.5-flash-lite",
                        "gemini-4-argon",
                    ],
                    false,
                ),
                Self::Groq => (
                    "Groq",
                    "Free tier and the fastest responses of any option here.",
                    "https://console.groq.com/keys",
                    true,
                    "openai/gpt-oss-120b",
                    vec![
                        "openai/gpt-oss-120b",
                        "openai/gpt-oss-20b",
                        "llama-3.3-70b-versatile",
                    ],
                    false,
                ),
                Self::DeepSeek => (
                    "DeepSeek",
                    "Very cheap, and strong at reasoning through a long meeting.",
                    "https://platform.deepseek.com/api_keys",
                    false,
                    "deepseek-flash",
                    vec!["deepseek-flash", "deepseek-v4-pro"],
                    false,
                ),
                Self::Anthropic => (
                    "Anthropic",
                    "High quality summaries and answers. No free tier.",
                    "https://console.anthropic.com/settings/keys",
                    false,
                    "claude-haiku-5-5",
                    vec![
                        "claude-haiku-5-5",
                        "claude-sonnet-5-5",
                        "claude-opus-5-5",
                        "claude-fable-5-1",
                    ],
                    false,
                ),
                Self::Deepgram => (
                    "Deepgram",
                    "A dedicated speech engine. Transcribes only; it cannot run the assistant.",
                    "https://console.deepgram.com/signup",
                    true,
                    "nova-3",
                    vec!["nova-3", "nova-2"],
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
            chat: self.chat(),
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
    fn a_provider_supports_speech_or_chat_or_both_but_is_never_useless() {
        for provider in ProviderId::ALL {
            assert!(
                provider.can_transcribe() || provider.can_chat(),
                "{provider:?} can do neither, so nothing could ever use it"
            );
        }
    }

    #[test]
    fn deepgram_transcribes_but_cannot_chat() {
        // A dedicated speech engine has no chat endpoint. Offering it to the
        // assistant would only produce a request that cannot land, so the
        // catalogue has to say so rather than the UI guessing.
        assert!(ProviderId::Deepgram.can_transcribe());
        assert!(!ProviderId::Deepgram.can_chat());
        assert!(ProviderId::Deepgram.info().chat.is_none());
    }

    #[test]
    fn deepgram_speaks_over_its_own_api_shape() {
        let speech = ProviderId::Deepgram
            .speech()
            .expect("Deepgram transcribes speech");
        assert_eq!(speech.api, SpeechApi::DeepgramListen);
        assert_eq!(speech.default_model, "nova-3");
    }

    #[test]
    fn every_speech_provider_names_a_default_model_it_also_offers() {
        for provider in ProviderId::ALL {
            let Some(speech) = provider.speech() else {
                continue;
            };
            // OpenAI-compatible is user-supplied, so its list is a suggestion.
            if provider == ProviderId::OpenAiCompatible {
                continue;
            }
            assert!(
                speech.models.contains(&speech.default_model),
                "{provider:?} defaults to a speech model it does not list"
            );
        }
    }

    /// Which pipeline a session runs is read out of the model name, so the
    /// reading has to be right for the names Google actually publishes.
    #[test]
    fn geminis_live_models_are_recognised_as_streaming_and_nothing_else_is() {
        assert!(ProviderId::Gemini.streams_speech(GEMINI_LIVE_MODEL));
        assert!(ProviderId::Gemini.streams_speech("gemini-2.0-flash-live-001"));
        assert!(ProviderId::Gemini.streams_speech("gemini-3.5-live-translate-preview"));

        // A model that only takes one request at a time must not be handed a
        // socket: it would connect, accept nothing, and caption silence.
        assert!(!ProviderId::Gemini.streams_speech("gemini-2.0-flash"));
        assert!(!ProviderId::Gemini.streams_speech(GEMINI_BATCH_MODEL));
        assert!(!ProviderId::Gemini.streams_speech("gemini-2.5-pro"));

        // No other provider speaks Gemini's socket protocol, whatever the model
        // happens to be called.
        for provider in ProviderId::ALL {
            if provider != ProviderId::Gemini {
                assert!(
                    !provider.streams_speech("live"),
                    "{provider:?} has no streaming API"
                );
            }
        }
    }

    /// The point of the change: Google's default is the live model, so a fresh
    /// install that picks Gemini gets word-by-word captions without knowing to
    /// go looking for them.
    #[test]
    fn gemini_transcribes_over_a_live_socket_by_default() {
        let speech = ProviderId::Gemini.speech().expect("Gemini transcribes");

        assert_eq!(speech.default_model, GEMINI_LIVE_MODEL);
        assert!(ProviderId::Gemini.streams_speech(&speech.default_model));
        assert_eq!(
            speech.models.first().map(String::as_str),
            Some(GEMINI_LIVE_MODEL),
            "the live model has to lead the list the UI shows"
        );
    }

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
