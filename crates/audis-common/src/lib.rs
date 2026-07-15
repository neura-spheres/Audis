//! Shared foundation for Audis.
//!
//! Base of the dependency graph: product identity, on-disk layout, error
//! presentation and IPC contracts. Knows nothing about audio devices, ASR
//! engines, providers or Tauri, which is what lets the higher layers stay
//! testable without a desktop shell.

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::all)]

pub mod error;
pub mod features;
pub mod files;
pub mod identity;
pub mod ipc;
pub mod language;
pub mod models;
pub mod paths;
pub mod providers;
pub mod session;
pub mod settings;
pub mod transcript;

pub use error::{AudisError, DiagnosticCode, Result, UserFacingError};
pub use features::{Feature, FeatureId, FeatureStatus};
pub use files::{DataCategory, DataCategoryGroup, DataFile, DataFileListing};
pub use ipc::{AppInfo, AudioSourceKind, DiagnosticWarning, events};
pub use language::Language;
pub use models::{InstalledModel, ModelDownloadProgress, ModelId, ModelInfo};
pub use paths::AppPaths;
pub use providers::{
    ProviderConfig, ProviderId, ProviderInfo, ProviderStatus, SpeechApi, SpeechSupport,
};
pub use session::{SessionMode, SessionState, SessionStatus};
pub use settings::{
    AudioSettings, CaptionSettings, Settings, TranscriptionEngine, TranscriptionSettings,
};
pub use transcript::{AsrState, AsrStatus, TranscriptSegment};
