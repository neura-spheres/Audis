//! Speech recognition through a cloud provider.

use std::time::Duration;

use audis_common::{Language, ProviderId, SpeechApi, SpeechSupport};
use base64::Engine as _;

use crate::engine::{AsrCapabilities, AsrEngine, AsrResult};
use crate::error::{AsrError, Result};
use crate::vad::Utterance;

/// How long to wait for a provider before giving up.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// Whisper's sample rate, which is what the endpointer produces.
const SAMPLE_RATE: u32 = crate::prepare::TARGET_SAMPLE_RATE;

/// A speech engine backed by a provider's API.
pub struct CloudEngine {
    provider: ProviderId,
    support: SpeechSupport,
    /// The provider's speech model, as the user chose it.
    model: String,
    /// Fully resolved base URL, with any trailing slash removed.
    base_url: String,
    api_key: String,
    client: reqwest::blocking::Client,
}

/// Written by hand, and never derived.
impl std::fmt::Debug for CloudEngine {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CloudEngine")
            .field("provider", &self.provider)
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

impl CloudEngine {
    /// Build an engine for `provider`.
    pub fn new(
        provider: ProviderId,
        model: String,
        endpoint: Option<String>,
        api_key: String,
    ) -> Result<Self> {
        let support = provider.speech().ok_or_else(|| AsrError::Recognition {
            detail: format!("{:?} cannot transcribe speech", provider),
        })?;

        if api_key.trim().is_empty() {
            return Err(AsrError::ProviderKeyMissing {
                provider: provider.info().name,
            });
        }

        let base_url = resolve_base_url(provider, &support, endpoint)?;

        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| AsrError::ProviderUnreachable {
                provider: provider.info().name,
                detail: error.to_string(),
            })?;

        let base_url = base_url.trim_end_matches('/').to_owned();
        tracing::info!(?provider, %model, %base_url, "cloud speech engine ready");

        Ok(Self {
            provider,
            support,
            model,
            base_url,
            api_key,
            client,
        })
    }

    fn provider_name(&self) -> String {
        self.provider.info().name
    }

    /// `POST /audio/transcriptions`, as OpenAI defined it and Groq copied it.
    fn transcribe_openai(&self, wav: Vec<u8>, language: Language) -> Result<String> {
        let part = reqwest::blocking::multipart::Part::bytes(wav)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|error| AsrError::Recognition {
                detail: error.to_string(),
            })?;

        let form = reqwest::blocking::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("language", language.code().to_owned())
            .text("response_format", "json".to_owned());

        let response = self
            .client
            .post(format!("{}/audio/transcriptions", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|error| AsrError::ProviderUnreachable {
                provider: self.provider_name(),
                detail: error.to_string(),
            })?;

        let status = response.status();
        let body = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(AsrError::ProviderRejected {
                provider: self.provider_name(),
                status: status.as_u16(),
                detail: body.chars().take(300).collect(),
            });
        }

        #[derive(serde::Deserialize)]
        struct TranscriptionResponse {
            text: String,
        }

        let parsed: TranscriptionResponse =
            serde_json::from_str(&body).map_err(|error| AsrError::Recognition {
                detail: format!("could not read the response: {error}"),
            })?;

        Ok(parsed.text)
    }

    /// Gemini's `generateContent`, with the audio inline.
    fn transcribe_gemini(&self, wav: Vec<u8>, language: Language) -> Result<String> {
        let audio = base64::engine::general_purpose::STANDARD.encode(&wav);
        let language_name = match language {
            Language::Indonesian => "Indonesian",
            Language::English => "English",
        };

        let body = serde_json::json!({
            "contents": [{
                "parts": [
                    { "text": format!(
                        "Transcribe this {language_name} audio exactly as spoken. Reply with the \
                         transcription only: no translation, no summary, no commentary, no \
                         quotation marks. If there is no speech, reply with nothing at all."
                    ) },
                    { "inline_data": { "mime_type": "audio/wav", "data": audio } }
                ]
            }],
            "generationConfig": {
                "temperature": 0.0
            }
        });

        let response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent",
                self.base_url, self.model
            ))
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .map_err(|error| AsrError::ProviderUnreachable {
                provider: self.provider_name(),
                detail: error.to_string(),
            })?;

        let status = response.status();
        let text = response.text().unwrap_or_default();

        if !status.is_success() {
            return Err(AsrError::ProviderRejected {
                provider: self.provider_name(),
                status: status.as_u16(),
                detail: text.chars().take(300).collect(),
            });
        }

        Ok(extract_gemini_text(&text))
    }
}

impl AsrEngine for CloudEngine {
    fn id(&self) -> &'static str {
        match self.provider {
            ProviderId::Groq => "groq-whisper",
            ProviderId::Gemini => "gemini",
            ProviderId::OpenAiCompatible => "openai-compatible",
            ProviderId::DeepSeek => "deepseek",
            ProviderId::Anthropic => "anthropic",
        }
    }

    fn capabilities(&self) -> AsrCapabilities {
        AsrCapabilities {
            offline: false,
            confidence: false,
            punctuation: true,
        }
    }

    fn transcribe(&mut self, utterance: &Utterance, language: Language) -> Result<AsrResult> {
        if utterance.samples.len() < 1_600 {
            return Ok(AsrResult {
                text: String::new(),
                language,
                confidence: None,
            });
        }

        let wav = encode_wav(&utterance.samples, SAMPLE_RATE);
        tracing::debug!(
            provider = ?self.provider,
            model = %self.model,
            bytes = wav.len(),
            "sending audio to provider"
        );

        let started = std::time::Instant::now();
        let text = match self.support.api {
            SpeechApi::OpenAiTranscriptions => self.transcribe_openai(wav, language)?,
            SpeechApi::GeminiInline => self.transcribe_gemini(wav, language)?,
        };
        tracing::debug!(
            provider = ?self.provider,
            ms = started.elapsed().as_millis(),
            chars = text.chars().count(),
            "provider transcription returned"
        );

        Ok(AsrResult {
            text: text.trim().to_owned(),
            language,
            confidence: None,
        })
    }
}

fn resolve_base_url(
    provider: ProviderId,
    support: &SpeechSupport,
    endpoint: Option<String>,
) -> Result<String> {
    let url = match (&support.base_url, endpoint) {
        (Some(fixed), _) => fixed.clone(),
        (None, Some(user)) if !user.trim().is_empty() => user.trim().to_owned(),
        (None, _) => {
            return Err(AsrError::Recognition {
                detail: format!(
                    "{} needs an endpoint, and none is set",
                    provider.info().name
                ),
            });
        }
    };
    Ok(url.trim_end_matches('/').to_owned())
}

/// What a fetched model list is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPurpose {
    /// Models that take audio in and return a transcript.
    Speech,
    /// Models that hold a conversation, for the assistant.
    Chat,
}

/// Ask a provider which models it currently offers for `purpose`.
pub fn fetch_models(
    provider: ProviderId,
    endpoint: Option<String>,
    api_key: String,
    purpose: ModelPurpose,
) -> Result<Vec<String>> {
    let support = provider.speech().ok_or_else(|| AsrError::Recognition {
        detail: format!("{provider:?} has no model list Audis can read"),
    })?;

    if api_key.trim().is_empty() {
        return Err(AsrError::ProviderKeyMissing {
            provider: provider.info().name,
        });
    }

    let base_url = resolve_base_url(provider, &support, endpoint)?;
    let name = provider.info().name;
    tracing::info!(?provider, ?purpose, %base_url, "fetching model list");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| AsrError::ProviderUnreachable {
            provider: name.clone(),
            detail: error.to_string(),
        })?;

    let models = match support.api {
        SpeechApi::OpenAiTranscriptions => {
            let body = get_json(
                client
                    .get(format!("{base_url}/models"))
                    .bearer_auth(&api_key),
                &name,
            )?;
            let mut ids: Vec<String> = body
                .get("data")
                .and_then(|data| data.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|entry| entry.get("id").and_then(|id| id.as_str()))
                        .filter(|id| openai_model_fits(id, purpose))
                        .map(str::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            ids.sort();
            ids
        }
        SpeechApi::GeminiInline => {
            let body = get_json(
                client
                    .get(format!("{base_url}/models"))
                    .header("x-goog-api-key", &api_key),
                &name,
            )?;
            let mut ids: Vec<String> = body
                .get("models")
                .and_then(|models| models.as_array())
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|entry| supports_generate_content(entry))
                        .filter_map(|entry| entry.get("name").and_then(|name| name.as_str()))
                        .map(|name| name.trim_start_matches("models/").to_owned())
                        .filter(|name| gemini_model_fits(name))
                        .collect()
                })
                .unwrap_or_default();
            ids.sort();
            ids
        }
    };

    if models.is_empty() {
        Ok(support.models)
    } else {
        Ok(models)
    }
}

fn get_json(
    request: reqwest::blocking::RequestBuilder,
    provider: &str,
) -> Result<serde_json::Value> {
    let response = request
        .send()
        .map_err(|error| AsrError::ProviderUnreachable {
            provider: provider.to_owned(),
            detail: error.to_string(),
        })?;

    let status = response.status();
    let text = response.text().unwrap_or_default();

    if !status.is_success() {
        return Err(AsrError::ProviderRejected {
            provider: provider.to_owned(),
            status: status.as_u16(),
            detail: text.chars().take(300).collect(),
        });
    }

    serde_json::from_str(&text).map_err(|error| AsrError::Recognition {
        detail: format!("could not read the model list: {error}"),
    })
}

/// Whether an OpenAI-shaped model id fits `purpose`.
///
/// The `/models` endpoint lists everything the key can reach: chat models,
/// speech models, embeddings, image and moderation endpoints. Speech keeps only
/// the transcription models; chat keeps the conversational ones and drops the
/// task-specific endpoints that cannot hold a conversation.
fn openai_model_fits(id: &str, purpose: ModelPurpose) -> bool {
    let id = id.to_lowercase();
    let is_speech = id.contains("whisper") || id.contains("transcribe");
    match purpose {
        ModelPurpose::Speech => is_speech,
        ModelPurpose::Chat => {
            const NOT_CHAT: [&str; 8] = [
                "whisper",
                "transcribe",
                "tts",
                "embed",
                "image",
                "dall-e",
                "moderation",
                "guard",
            ];
            !NOT_CHAT.iter().any(|bad| id.contains(bad))
        }
    }
}

/// Whether a Gemini model is one of the general multimodal `flash`/`pro`
/// models, which serve both transcription and chat.
///
/// Gemini's `/models` lists image generators, text-to-speech, embeddings and
/// other tools alongside them, none of which can transcribe or converse, so
/// those are rejected by the purpose in the name.
fn gemini_model_fits(name: &str) -> bool {
    let name = name.to_lowercase();
    if !name.starts_with("gemini") {
        return false;
    }
    const NOT_GENERAL: [&str; 7] = [
        "image",
        "tts",
        "embedding",
        "aqa",
        "computer-use",
        "vision",
        "-live",
    ];
    if NOT_GENERAL.iter().any(|bad| name.contains(bad)) {
        return false;
    }
    name.contains("flash") || name.contains("pro")
}

fn supports_generate_content(entry: &serde_json::Value) -> bool {
    entry
        .get("supportedGenerationMethods")
        .and_then(|methods| methods.as_array())
        .map(|methods| {
            methods
                .iter()
                .any(|method| method.as_str() == Some("generateContent"))
        })
        .unwrap_or(false)
}

/// Pull the transcription out of a Gemini response.
fn extract_gemini_text(body: &str) -> String {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(body) else {
        return String::new();
    };

    json.get("candidates")
        .and_then(|candidates| candidates.get(0))
        .and_then(|candidate| candidate.get("content"))
        .and_then(|content| content.get("parts"))
        .and_then(|parts| parts.get(0))
        .and_then(|part| part.get("text"))
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_owned()
}

/// Wrap 16 kHz mono samples in a WAV container.
fn encode_wav(samples: &[f32], sample_rate: u32) -> Vec<u8> {
    let channels: u16 = 1;
    let bits: u16 = 16;
    let byte_rate = sample_rate * u32::from(channels) * u32::from(bits / 8);
    let block_align = channels * (bits / 8);
    let data_len = (samples.len() * 2) as u32;

    let mut wav = Vec::with_capacity(44 + data_len as usize);

    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits.to_le_bytes());

    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let scaled = (clamped * f32::from(i16::MAX)) as i16;
        wav.extend_from_slice(&scaled.to_le_bytes());
    }

    wav
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn only_multimodal_gemini_models_are_offered_for_speech() {
        assert!(gemini_model_fits("gemini-2.0-flash"));
        assert!(gemini_model_fits("gemini-2.5-pro"));
        assert!(gemini_model_fits("gemini-3-flash-preview"));
        assert!(gemini_model_fits("gemini-2.0-flash-lite"));

        for rejected in [
            "gemini-3-pro-image",
            "gemini-2.5-flash-image",
            "gemini-2.5-flash-preview-tts",
            "gemini-2.5-computer-use-preview-10-2025",
            "text-embedding-004",
            "aqa",
        ] {
            assert!(
                !gemini_model_fits(rejected),
                "{rejected} cannot transcribe and must not be offered"
            );
        }
    }

    #[test]
    fn openai_speech_filter_keeps_only_whisper_and_transcribe() {
        assert!(openai_model_fits("whisper-large-v3", ModelPurpose::Speech));
        assert!(openai_model_fits("gpt-4o-transcribe", ModelPurpose::Speech));
        assert!(!openai_model_fits("gpt-4o", ModelPurpose::Speech));
        assert!(!openai_model_fits("llama-3.3-70b", ModelPurpose::Speech));

        assert!(openai_model_fits("gpt-4o", ModelPurpose::Chat));
        assert!(openai_model_fits(
            "llama-3.3-70b-versatile",
            ModelPurpose::Chat
        ));
        assert!(!openai_model_fits("whisper-large-v3", ModelPurpose::Chat));
        assert!(!openai_model_fits(
            "text-embedding-3-small",
            ModelPurpose::Chat
        ));
    }

    #[test]
    fn a_wav_has_a_header_every_provider_can_read() {
        let wav = encode_wav(&[0.0, 0.5, -0.5], 16_000);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 3 * 2, "header plus one i16 per sample");

        let riff_len = u32::from_le_bytes(wav[4..8].try_into().unwrap());
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap());
        assert_eq!(riff_len as usize, wav.len() - 8);
        assert_eq!(data_len as usize, wav.len() - 44);

        let rate = u32::from_le_bytes(wav[24..28].try_into().unwrap());
        assert_eq!(rate, 16_000);
    }

    /// A sample above full scale must clip, not wrap.
    #[test]
    fn loud_audio_clips_rather_than_wrapping_to_a_crack() {
        let wav = encode_wav(&[2.0, -2.0], 16_000);
        let first = i16::from_le_bytes(wav[44..46].try_into().unwrap());
        let second = i16::from_le_bytes(wav[46..48].try_into().unwrap());

        assert_eq!(first, i16::MAX);
        assert_eq!(second, -i16::MAX);
    }

    #[test]
    fn a_provider_without_speech_is_refused_at_construction() {
        let error = CloudEngine::new(
            ProviderId::Anthropic,
            "claude".to_owned(),
            None,
            "key".to_owned(),
        );
        assert!(error.is_err(), "Anthropic has no speech API");
    }

    /// The message must name the missing key, not fail obscurely mid-sentence.
    #[test]
    fn an_empty_key_is_refused_before_any_audio_is_sent() {
        let error = CloudEngine::new(
            ProviderId::Groq,
            "whisper-large-v3".to_owned(),
            None,
            "   ".to_owned(),
        )
        .expect_err("an empty key must be refused");

        assert!(matches!(error, AsrError::ProviderKeyMissing { .. }));
    }

    /// A fixed-endpoint provider must ignore a stale user endpoint rather than
    #[test]
    fn a_stale_endpoint_cannot_redirect_a_fixed_provider() {
        let engine = CloudEngine::new(
            ProviderId::Groq,
            "whisper-large-v3".to_owned(),
            Some("https://somewhere-else.example".to_owned()),
            "key".to_owned(),
        )
        .expect("groq builds");

        assert_eq!(engine.base_url, "https://api.groq.com/openai/v1");
    }

    #[test]
    fn a_user_endpoint_is_used_when_the_provider_has_none() {
        let engine = CloudEngine::new(
            ProviderId::OpenAiCompatible,
            "whisper-1".to_owned(),
            Some("https://api.openai.com/v1/".to_owned()),
            "key".to_owned(),
        )
        .expect("openai-compatible builds");

        assert_eq!(engine.base_url, "https://api.openai.com/v1");
    }

    /// The credential design keeps keys out of settings, logs and exports. A
    #[test]
    fn debug_never_prints_the_api_key() {
        let engine = CloudEngine::new(
            ProviderId::Groq,
            "whisper-large-v3".to_owned(),
            None,
            "sk-super-secret-value".to_owned(),
        )
        .expect("groq builds");

        let printed = format!("{engine:?}");

        assert!(
            !printed.contains("sk-super-secret-value"),
            "Debug leaked the API key: {printed}"
        );
        assert!(printed.contains("<redacted>"));
        assert!(printed.contains("Groq"));
    }

    #[test]
    fn a_gemini_response_yields_its_text() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"halo dunia"}]}}]}"#;
        assert_eq!(extract_gemini_text(body), "halo dunia");
    }

    /// Gemini returns no candidate when it has nothing to say. That is silence,
    #[test]
    fn a_gemini_response_without_a_candidate_is_treated_as_silence() {
        assert_eq!(extract_gemini_text(r#"{"candidates":[]}"#), "");
        assert_eq!(extract_gemini_text("not json at all"), "");
    }
}
