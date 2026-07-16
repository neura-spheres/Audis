//! The local Whisper engine, via whisper.cpp.

use std::path::Path;

use audis_common::Language;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::engine::{AsrCapabilities, AsrEngine, AsrResult};
use crate::error::{AsrError, Result};
use crate::vad::Utterance;

/// Above this, the model believes the audio contains no speech.
const NO_SPEECH_THRESHOLD: f32 = 0.6;

/// Temperature step used when a decode fails the checks below.
const TEMPERATURE_INC: f32 = 0.2;

/// Above this entropy, the decode has collapsed into repetition.
const ENTROPY_THRESHOLD: f32 = 2.4;

/// Below this average log probability, the model was guessing.
const LOGPROB_THRESHOLD: f32 = -1.0;

/// Whisper running locally on the CPU.
pub struct WhisperEngine {
    context: WhisperContext,
    threads: i32,
}

impl WhisperEngine {
    /// Load a ggml model from disk.
    pub fn load(model_path: &Path) -> Result<Self> {
        if !model_path.is_file() {
            return Err(AsrError::ModelMissing {
                path: model_path.display().to_string(),
            });
        }

        let path = model_path.to_str().ok_or_else(|| AsrError::ModelMissing {
            path: model_path.display().to_string(),
        })?;

        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|error| AsrError::ModelLoad {
                path: model_path.display().to_string(),
                detail: error.to_string(),
            })?;

        let threads = (std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(4)
            .saturating_sub(1))
        .clamp(1, 8) as i32;

        tracing::info!(model = %model_path.display(), threads, "whisper model loaded");

        Ok(Self { context, threads })
    }
}

impl AsrEngine for WhisperEngine {
    fn id(&self) -> &'static str {
        "whisper-local"
    }

    fn capabilities(&self) -> AsrCapabilities {
        AsrCapabilities {
            offline: true,
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

        let mut state = self
            .context
            .create_state()
            .map_err(|error| AsrError::Recognition {
                detail: error.to_string(),
            })?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        params.set_language(Some(language.code()));
        params.set_translate(false);

        params.set_temperature(0.0);
        params.set_temperature_inc(TEMPERATURE_INC);
        params.set_entropy_thold(ENTROPY_THRESHOLD);
        params.set_logprob_thold(LOGPROB_THRESHOLD);
        params.set_no_speech_thold(NO_SPEECH_THRESHOLD);

        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_suppress_blank(true);

        state
            .full(params, &utterance.samples)
            .map_err(|error| AsrError::Recognition {
                detail: error.to_string(),
            })?;

        let mut text = String::new();

        for segment in state.as_iter() {
            if segment.no_speech_probability() > NO_SPEECH_THRESHOLD {
                tracing::debug!(
                    probability = segment.no_speech_probability(),
                    "dropped a segment the model considers non-speech"
                );
                continue;
            }

            match segment.to_str_lossy() {
                Ok(part) => text.push_str(&part),
                Err(error) => {
                    tracing::warn!(%error, "a segment could not be decoded as text");
                }
            }
        }

        Ok(AsrResult {
            text: text.trim().to_owned(),
            language,
            confidence: None,
        })
    }
}
