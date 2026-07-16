//! Speech recognition for Audis.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod chat;
pub mod cloud;
pub mod engine;
pub mod error;
pub mod prepare;
pub mod vad;

#[cfg(feature = "local-whisper")]
pub mod whisper;

pub use chat::chat;
pub use cloud::{CloudEngine, ModelPurpose, fetch_models};
pub use engine::{AsrCapabilities, AsrEngine, AsrResult};
pub use error::{AsrError, Result};
pub use prepare::{Resampler, TARGET_SAMPLE_RATE, downmix_to_mono, prepare};
pub use vad::{EndpointConfig, EndpointEvent, Endpointer, Utterance};

#[cfg(feature = "local-whisper")]
pub use whisper::WhisperEngine;

/// Load the local engine for a model file.
#[cfg(feature = "local-whisper")]
pub fn load_local_engine(model_path: &std::path::Path) -> Result<Box<dyn AsrEngine>> {
    Ok(Box::new(WhisperEngine::load(model_path)?))
}

/// Load the local engine for a model file.
#[cfg(not(feature = "local-whisper"))]
pub fn load_local_engine(_model_path: &std::path::Path) -> Result<Box<dyn AsrEngine>> {
    Err(AsrError::NoLocalEngine)
}
