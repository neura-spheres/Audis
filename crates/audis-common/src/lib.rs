//! Shared foundation for Audis.

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
    ChatApi, ChatSupport, ProviderConfig, ProviderId, ProviderInfo, ProviderStatus, SpeechApi,
    SpeechSupport,
};
pub use session::{SessionMode, SessionState, SessionStatus};
pub use settings::{
    AssistantContext, AssistantSettings, AudioSettings, CaptionSettings, RecordingSettings,
    Settings, SpeakerSettings, TranscriptionEngine, TranscriptionSettings, UpdateChannel,
    UpdateSettings,
};
pub use transcript::{
    AsrState, AsrStatus, MeetingUpdate, SegmentRevision, SpeakerUpdate, TranscriptSegment,
};
