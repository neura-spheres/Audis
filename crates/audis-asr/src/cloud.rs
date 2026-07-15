//! Speech recognition through a cloud provider.
//!
//! Why this exists: no Whisper model that keeps up with live speech on a normal
//! CPU is good at Indonesian. Base decodes at 0.61x real time and misreads
//! ordinary words; the models that get Indonesian right decode at two to five
//! times real time, which is not captioning at all. A provider running
//! `whisper-large-v3` on their own hardware breaks that deadlock: the accuracy
//! of a large model, faster than local Base.
//!
//! What it costs is the thing Audis otherwise promises. Using this sends the
//! user's audio over the internet to a company, so it is never a default and
//! never inferred — see `TranscriptionEngine`.
//!
//! Blocking HTTP on purpose. [`AsrEngine::transcribe`] is sync and runs on a
//! dedicated worker thread, so there is no async runtime to block and no reason
//! to colour the whole trait async for one implementation.

use std::time::Duration;

use audis_common::{Language, ProviderId, SpeechApi, SpeechSupport};
use base64::Engine as _;

use crate::engine::{AsrCapabilities, AsrEngine, AsrResult};
use crate::error::{AsrError, Result};
use crate::vad::Utterance;

/// How long to wait for a provider before giving up.
///
/// Generous enough for a slow connection and a long sentence, short enough that
/// a hung request does not stall every caption behind it: the recogniser is a
/// single thread, so this is the worst case for one utterance holding it.
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
///
/// A derived `Debug` would print `api_key` in full the first time anyone logged
/// this struct or unwrapped a `Result` containing it. The whole credential
/// design keeps the key out of settings, logs and support bundles; one careless
/// `{:?}` would undo that.
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
    ///
    /// `endpoint` is only used by providers whose URL the user supplies; for
    /// the rest it is ignored in favour of the catalogue's own base URL, so a
    /// stale endpoint left over from another provider cannot misdirect audio.
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

        let base_url = match (&support.base_url, endpoint) {
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

        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|error| AsrError::ProviderUnreachable {
                provider: provider.info().name,
                detail: error.to_string(),
            })?;

        Ok(Self {
            provider,
            support,
            model,
            base_url: base_url.trim_end_matches('/').to_owned(),
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
            // Forced, exactly as locally: Audis knows which of its two
            // languages is being spoken, so detection buys nothing and can
            // pick wrong on a code-switched sentence.
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
                // The provider's message, trimmed: it can be a page of JSON and
                // this ends up in a log and a support bundle.
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
    ///
    /// Gemini is a general model asked to transcribe rather than a speech
    /// engine, so the prompt does real work here: without it the model answers
    /// questions it hears, summarises, or translates.
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
                // Deterministic: this is a transcription, not a creative task.
                "temperature": 0.0
            }
        });

        let response = self
            .client
            .post(format!(
                "{}/models/{}:generateContent",
                self.base_url, self.model
            ))
            // Gemini takes the key in a header rather than a bearer token.
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
            // The whole point of this engine, and the whole cost of it.
            offline: false,
            // None of these return a confidence Audis has validated, so it
            // claims none rather than inventing one.
            confidence: false,
            punctuation: true,
        }
    }

    fn transcribe(&mut self, utterance: &Utterance, language: Language) -> Result<AsrResult> {
        // Too short to be a word, and a request costs a round trip and quota.
        if utterance.samples.len() < 1_600 {
            return Ok(AsrResult {
                text: String::new(),
                language,
                confidence: None,
            });
        }

        let wav = encode_wav(&utterance.samples, SAMPLE_RATE);

        let text = match self.support.api {
            SpeechApi::OpenAiTranscriptions => self.transcribe_openai(wav, language)?,
            SpeechApi::GeminiInline => self.transcribe_gemini(wav, language)?,
        };

        Ok(AsrResult {
            text: text.trim().to_owned(),
            language,
            confidence: None,
        })
    }
}

/// Pull the transcription out of a Gemini response.
///
/// Tolerant by design: a missing candidate means the model refused or the audio
/// was silence, and an empty caption is the right answer to both. Failing the
/// whole utterance would turn a shrug into an error dialog.
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
///
/// Every one of these APIs wants a file rather than raw samples, and WAV is the
/// only format all of them accept without a codec. 16-bit PCM is what Whisper
/// works in anyway, so this loses nothing.
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
        // Clamped before scaling: a sample above 1.0 would wrap to a loud crack
        // at the opposite polarity, which is worse than clipping.
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
    fn a_wav_has_a_header_every_provider_can_read() {
        let wav = encode_wav(&[0.0, 0.5, -0.5], 16_000);

        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav.len(), 44 + 3 * 2, "header plus one i16 per sample");

        // The declared sizes must match the real ones, or a strict decoder
        // rejects the file and the provider returns an unhelpful 400.
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
    /// send audio somewhere the user did not intend.
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

        // The trailing slash is removed so paths do not end up doubled.
        assert_eq!(engine.base_url, "https://api.openai.com/v1");
    }

    /// The credential design keeps keys out of settings, logs and exports. A
    /// derived `Debug` here would put one back the first time anything logged
    /// this struct.
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
        // Still useful for diagnosis: it says who and where.
        assert!(printed.contains("Groq"));
    }

    #[test]
    fn a_gemini_response_yields_its_text() {
        let body = r#"{"candidates":[{"content":{"parts":[{"text":"halo dunia"}]}}]}"#;
        assert_eq!(extract_gemini_text(body), "halo dunia");
    }

    /// Gemini returns no candidate when it has nothing to say. That is silence,
    /// not a failure.
    #[test]
    fn a_gemini_response_without_a_candidate_is_treated_as_silence() {
        assert_eq!(extract_gemini_text(r#"{"candidates":[]}"#), "");
        assert_eq!(extract_gemini_text("not json at all"), "");
    }
}
