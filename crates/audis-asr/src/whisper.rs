//! The local Whisper engine, via whisper.cpp.
//!
//! Free, offline, no account, and good at both of Audis' languages. See
//! ADR-011 for why this is the default and what it costs to build.

use std::path::Path;

use audis_common::Language;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::engine::{AsrCapabilities, AsrEngine, AsrResult};
use crate::error::{AsrError, Result};
use crate::vad::Utterance;

/// Above this, the model believes the audio contains no speech.
///
/// 0.6 rather than 0.5: the cost of dropping a real quiet word is a missing
/// caption, while the cost of keeping a hallucination is text the user never
/// said appearing in their transcript. The second is worse, but only just, so
/// this sits slightly on the permissive side of the middle.
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
    ///
    /// Loading is slow (hundreds of milliseconds to seconds) and allocates the
    /// whole model, so this happens once when a session starts, never per
    /// utterance.
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

        // Leave a core for the audio callback and the UI. Whisper will happily
        // saturate every core and make the rest of the app stutter.
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
            // whisper.cpp exposes token probabilities, but turning those into a
            // number a user can trust is its own problem. Claiming confidence
            // we have not validated would be worse than admitting we have none.
            confidence: false,
            punctuation: true,
        }
    }

    fn transcribe(&mut self, utterance: &Utterance, language: Language) -> Result<AsrResult> {
        // Whisper pads to 30 seconds internally, so anything shorter than a
        // moment of speech is mostly padding and decodes to noise.
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

        // Greedy, and this is not a compromise made lightly. Beam search is
        // more accurate, especially for Indonesian, and it was tried: measured
        // on a 12-core CPU with a release build, Whisper Base decodes 3.4s of
        // speech in 2.1s greedy (0.61x real time) and 7.1s with beam search
        // (2.06x). Anything above 1.0x is not slower captions, it is captions
        // that fall further behind every sentence until they are worthless, and
        // one recogniser thread serves both audio sources.
        //
        // Accuracy that arrives after the meeting is not accuracy. The cheap
        // gains — temperature fallback and the degeneracy thresholds below —
        // are kept because they only cost time on decodes that were bad anyway.
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Forcing the language is the whole point of supporting exactly two.
        // Detection costs a pass and can pick the wrong one mid-meeting when
        // someone code-switches between Indonesian and English.
        params.set_language(Some(language.code()));
        params.set_translate(false);

        // Temperature fallback: decode at 0 for a deterministic best guess, and
        // only retry hotter when the result looks degenerate by the two checks
        // below. Without an increment there is no fallback at all, so a single
        // bad decode stands as final — which shows up as a confidently wrong
        // caption rather than a second attempt.
        params.set_temperature(0.0);
        params.set_temperature_inc(TEMPERATURE_INC);
        // Whisper's own signals that a decode collapsed into repetition or
        // guesswork. These are whisper.cpp's defaults; they are off unless set.
        params.set_entropy_thold(ENTROPY_THRESHOLD);
        params.set_logprob_thold(LOGPROB_THRESHOLD);
        // Give the decoder the same non-speech threshold the segment filter
        // below uses, so the two cannot disagree about what counts as silence.
        params.set_no_speech_thold(NO_SPEECH_THRESHOLD);

        params.set_n_threads(self.threads);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // The endpointer already decided where this utterance begins and ends,
        // so Whisper must not add its own silence suppression on top.
        params.set_suppress_blank(true);

        state
            .full(params, &utterance.samples)
            .map_err(|error| AsrError::Recognition {
                detail: error.to_string(),
            })?;

        let mut text = String::new();

        for segment in state.as_iter() {
            // Whisper hallucinates on near-silence: it will confidently emit
            // "Thank you." or subtitle credits from its training data when
            // handed room tone. The endpointer keeps most of that out, but a
            // quiet utterance still slips through. This probability is the
            // model's own signal that it heard nothing, so trust it.
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
                    // One bad segment should not lose the whole utterance.
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
