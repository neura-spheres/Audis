//! Speech recognition for Audis.
//!
//! The live path is: capture, prepare (downmix and resample), detect voice
//! activity, endpoint into utterances, recognise, emit. Whisper is not a
//! streaming engine, so [`vad::Endpointer`] is what makes live captions
//! possible: it decides when an utterance ended and hands that to the engine.
//!
//! Audis recognises exactly two languages, Indonesian and English, and always
//! tells the engine which one. See [`audis_common::Language`].

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod cloud;
pub mod engine;
pub mod error;
pub mod prepare;
pub mod vad;

#[cfg(feature = "local-whisper")]
pub mod whisper;

pub use cloud::CloudEngine;
pub use engine::{AsrCapabilities, AsrEngine, AsrResult};
pub use error::{AsrError, Result};
pub use prepare::{Resampler, TARGET_SAMPLE_RATE, downmix_to_mono, prepare};
pub use vad::{EndpointConfig, EndpointEvent, Endpointer, Utterance};

#[cfg(feature = "local-whisper")]
pub use whisper::WhisperEngine;

/// Load the local engine for a model file.
///
/// Returns [`AsrError::NoLocalEngine`] when built without `local-whisper`, so
/// the app can report that honestly instead of failing to compile.
#[cfg(feature = "local-whisper")]
pub fn load_local_engine(model_path: &std::path::Path) -> Result<Box<dyn AsrEngine>> {
    Ok(Box::new(WhisperEngine::load(model_path)?))
}

/// Load the local engine for a model file.
#[cfg(not(feature = "local-whisper"))]
pub fn load_local_engine(_model_path: &std::path::Path) -> Result<Box<dyn AsrEngine>> {
    Err(AsrError::NoLocalEngine)
}
