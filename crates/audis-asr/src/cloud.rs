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

    /// Deepgram's `POST /listen`, with the audio as the raw request body.
    ///
    /// A purpose-built speech engine, so unlike the chat-model path there is no
    /// prompt to get wrong and nothing to talk it out of role.
    fn transcribe_deepgram(&self, wav: Vec<u8>, language: Language) -> Result<String> {
        // smart_format gives punctuation and capitalisation, which captions
        // need to read as sentences rather than a stream of words.
        let url = format!(
            "{}/listen?model={}&language={}&smart_format=true",
            self.base_url,
            self.model.trim(),
            language.code()
        );

        let response = self
            .client
            .post(url)
            .header("Authorization", format!("Token {}", self.api_key))
            .header("Content-Type", "audio/wav")
            .body(wav)
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
        struct Response {
            results: Results,
        }
        #[derive(serde::Deserialize)]
        struct Results {
            channels: Vec<Channel>,
        }
        #[derive(serde::Deserialize)]
        struct Channel {
            alternatives: Vec<Alternative>,
        }
        #[derive(serde::Deserialize)]
        struct Alternative {
            transcript: String,
        }

        let parsed: Response =
            serde_json::from_str(&body).map_err(|error| AsrError::Recognition {
                detail: format!("could not read the response: {error}"),
            })?;

        // Silence comes back as an empty alternatives list rather than an error.
        Ok(parsed
            .results
            .channels
            .into_iter()
            .next()
            .and_then(|channel| channel.alternatives.into_iter().next())
            .map(|alternative| alternative.transcript)
            .unwrap_or_default())
    }

    /// Gemini's `generateContent`, with the audio inline.
    fn transcribe_gemini(&self, wav: Vec<u8>, language: Language) -> Result<String> {
        let audio = base64::engine::general_purpose::STANDARD.encode(&wav);
        let language_name = match language {
            Language::Indonesian => "Indonesian",
            Language::English => "English",
        };

        // Gemini is a chat model wearing a transcriber's hat, and it will fall
        // back to being an assistant ("Hi, how can I help you?") given half a
        // chance. Two things keep it in role: a system instruction it cannot
        // talk its way out of, and a sentinel to emit for silence — asked to
        // "reply with nothing" it would rather greet the user than say nothing.
        let body = serde_json::json!({
            "systemInstruction": {
                "parts": [{ "text": format!(
                    "You are a speech-to-text engine, not an assistant. You transcribe \
                     {language_name} audio verbatim and do nothing else. Never greet, never \
                     answer, never explain, never apologise, never describe the audio. Output \
                     only the exact words spoken, with no quotation marks and no added \
                     punctuation beyond what is spoken. If the audio contains no intelligible \
                     speech, output exactly {NO_SPEECH} and nothing else."
                ) }]
            },
            "contents": [{
                "parts": [
                    { "text": format!("Transcribe this {language_name} audio verbatim.") },
                    { "inline_data": { "mime_type": "audio/wav", "data": audio } }
                ]
            }],
            "generationConfig": {
                "temperature": 0.0,
                "maxOutputTokens": 256
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
            ProviderId::Deepgram => "deepgram",
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
        let quiet = AsrResult {
            text: String::new(),
            language,
            confidence: None,
        };

        // Never spend a round-trip on audio with no speech in it. Besides the
        // waste, a chat-based provider handed silence tends to invent a reply
        // rather than return nothing.
        if utterance.samples.len() < MIN_SAMPLES || rms(&utterance.samples) < MIN_SPEECH_RMS {
            return Ok(quiet);
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
            SpeechApi::DeepgramListen => self.transcribe_deepgram(wav, language)?,
        };
        tracing::debug!(
            provider = ?self.provider,
            ms = started.elapsed().as_millis(),
            chars = text.chars().count(),
            "provider transcription returned"
        );

        let text = text.trim();
        if text.is_empty() || text.contains(NO_SPEECH) {
            return Ok(quiet);
        }

        Ok(AsrResult {
            text: text.to_owned(),
            language,
            confidence: None,
        })
    }
}

/// What a chat-based provider is told to emit when it hears no speech.
///
/// Filtered out before anything reaches a caption. Deliberately not a word
/// anyone would say out loud, so a real transcript can never collide with it.
const NO_SPEECH: &str = "<no_speech>";

/// Shortest audio worth sending: below this there is nothing to transcribe.
const MIN_SAMPLES: usize = SAMPLE_RATE as usize / 5;

/// Loudness below which audio is treated as silence rather than speech.
///
/// Matches the endpointer's own gate, so the two agree on what silence is.
const MIN_SPEECH_RMS: f32 = 0.008;

/// Root-mean-square loudness of a block of samples.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|sample| sample * sample).sum();
    (sum / samples.len() as f32).sqrt()
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
        SpeechApi::DeepgramListen => {
            // The speech models are the `stt` list, so there is no guessing from
            // the name. Deepgram has nothing to offer the assistant.
            if matches!(purpose, ModelPurpose::Chat) {
                Vec::new()
            } else {
                let body = get_json(
                    client
                        .get(format!("{base_url}/models"))
                        .header("Authorization", format!("Token {api_key}")),
                    &name,
                )?;
                let mut ids: Vec<String> = body
                    .get("stt")
                    .and_then(|stt| stt.as_array())
                    .map(|entries| entries.iter().filter_map(deepgram_speech_model).collect())
                    .unwrap_or_default();
                ids.sort();
                ids.dedup();
                // Newest generation first, so the best answer leads.
                ids.reverse();
                ids
            }
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

/// The general Nova model for one entry in Deepgram's catalogue, if it is one.
///
/// Deepgram lists everything it has ever served: legacy tiers (`base`,
/// `enhanced`), hosted Whisper mirrors (`large`, `medium`), and per-domain
/// variants (`nova-2-medical`, `nova-2-drivethru`, `nova-2-atc`). Offering all
/// of that would be asking someone to pick a speech model out of a catalogue
/// they have no way to judge, when only one answer is ever right for live
/// captions: the current general Nova.
///
/// So this keeps `nova-<n>` and `nova-<n>-general` and drops the rest, reporting
/// the short form both spellings resolve to. A new generation appears here on
/// its own, without a release.
fn deepgram_speech_model(entry: &serde_json::Value) -> Option<String> {
    let canonical = entry
        .get("canonical_name")
        .and_then(|name| name.as_str())?
        .trim();

    let generation = canonical.strip_prefix("nova-")?;
    let generation = generation.strip_suffix("-general").unwrap_or(generation);

    if generation.is_empty() || !generation.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }

    Some(format!("nova-{generation}"))
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

    /// One entry as Deepgram's `stt` list carries it.
    fn stt_entry(canonical: &str) -> serde_json::Value {
        serde_json::json!({ "canonical_name": canonical, "name": canonical })
    }

    #[test]
    fn deepgram_filter_keeps_the_general_nova_models() {
        assert_eq!(
            deepgram_speech_model(&stt_entry("nova-3-general")).as_deref(),
            Some("nova-3")
        );
        assert_eq!(
            deepgram_speech_model(&stt_entry("nova-2-general")).as_deref(),
            Some("nova-2")
        );
        // Both spellings are accepted by the API and mean the same model.
        assert_eq!(
            deepgram_speech_model(&stt_entry("nova-3")).as_deref(),
            Some("nova-3")
        );
    }

    #[test]
    fn deepgram_filter_drops_everything_nobody_should_pick_for_captions() {
        // Every one of these is really in Deepgram's catalogue, and every one of
        // them was in the dropdown before this filter existed.
        for rejected in [
            "nova-2-atc",
            "nova-2-meeting",
            "nova-2-phonecall",
            "nova-2-medical",
            "nova-2-drivethru",
            "nova-2-automotive",
            "nova-2-finance",
            "base",
            "enhanced",
            "large",
            "medium",
            "small",
            "phoneme",
            "conversationalai",
            "general-dQw4w9WgXcQ",
            "general-polaris",
        ] {
            assert!(
                deepgram_speech_model(&stt_entry(rejected)).is_none(),
                "{rejected} should not be offered as a speech model"
            );
        }
    }

    #[test]
    fn deepgram_filter_picks_up_a_future_generation_without_a_release() {
        assert_eq!(
            deepgram_speech_model(&stt_entry("nova-4-general")).as_deref(),
            Some("nova-4")
        );
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
