//! Error types and their user-facing presentation.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Convenience alias for fallible Audis operations.
pub type Result<T, E = AudisError> = std::result::Result<T, E>;

/// Error type for the shared layer.
#[derive(Debug, thiserror::Error)]
pub enum AudisError {
    /// A setting or environment value could not be understood.
    #[error("configuration error: {detail}")]
    Configuration {
        /// What was wrong.
        detail: String,
    },

    /// A filesystem operation failed.
    #[error("{detail} ({path})")]
    Io {
        /// The path being operated on.
        path: PathBuf,
        /// What Audis was attempting.
        detail: String,
        /// The underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// JSON could not be read or written.
    #[error("could not serialise or deserialise {context}")]
    Serialization {
        /// What was being converted, for example "settings.json".
        context: String,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// An argument from the frontend failed validation.
    #[error("invalid argument {field}: {detail}")]
    InvalidArgument {
        /// The offending field, as named in the IPC contract.
        field: String,
        /// Why it was rejected.
        detail: String,
    },
}

/// A stable code shown to users and written to logs, so support can map a
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DiagnosticCode {
    /// A setting could not be read; defaults were used.
    ConfigInvalid,
    /// The data directory or a file within it could not be reached.
    StorageUnavailable,
    /// Saved data could not be parsed.
    DataSerialization,
    /// An IPC argument failed validation.
    InvalidRequest,
    /// Fallback for errors with no more specific code.
    Unexpected,
}

impl DiagnosticCode {
    /// Stable string form, for example `AUDIS-CONFIG-INVALID`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConfigInvalid => "AUDIS-CONFIG-INVALID",
            Self::StorageUnavailable => "AUDIS-STORAGE-UNAVAILABLE",
            Self::DataSerialization => "AUDIS-DATA-SERIALIZATION",
            Self::InvalidRequest => "AUDIS-INVALID-REQUEST",
            Self::Unexpected => "AUDIS-UNEXPECTED",
        }
    }
}

/// The error shape the UI renders. This is what crosses the IPC boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserFacingError {
    /// Short title, for example "Computer audio could not start".
    pub title: String,
    /// Plain explanation with no jargon.
    pub explanation: String,
    /// Whether the user's data survived. Required, never implied: the worst
    pub data_preserved: bool,
    /// The most useful thing the user can do next.
    pub suggested_action: String,
    /// Detail shown only behind a disclosure.
    pub technical_details: Option<String>,
    /// Code for support and log correlation.
    pub diagnostic_code: DiagnosticCode,
}

impl AudisError {
    /// Translate an error into the message a user should read.
    pub fn to_user_facing(&self) -> UserFacingError {
        let technical_details = Some(self.to_string());

        match self {
            AudisError::Configuration { .. } => UserFacingError {
                title: "Audis could not read its configuration".to_owned(),
                explanation: "A setting could not be understood, so Audis used its defaults \
                              instead. Your sessions and recordings were not affected."
                    .to_owned(),
                data_preserved: true,
                suggested_action:
                    "Open Settings and check your storage location, then restart Audis.".to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::ConfigInvalid,
            },

            AudisError::Io { .. } => UserFacingError {
                title: "Audis could not reach its storage folder".to_owned(),
                explanation: "A file or folder Audis needs could not be opened. Any session \
                              already saved is still on disk and was not changed."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Check that the Audis storage folder exists and is writable, \
                                   then try again."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::StorageUnavailable,
            },

            AudisError::Serialization { .. } => UserFacingError {
                title: "Audis could not read some saved data".to_owned(),
                explanation: "A saved file could not be understood. The original file was left \
                              exactly as it is and nothing was overwritten."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Export a diagnostic bundle from Settings so this can be \
                                   investigated."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::DataSerialization,
            },

            AudisError::InvalidArgument { .. } => UserFacingError {
                title: "Audis could not complete that request".to_owned(),
                explanation: "The request was not valid, so Audis stopped before making any \
                              change. Nothing was modified."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Try the action again. If it keeps failing, restart Audis."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::InvalidRequest,
            },
        }
    }
}

impl From<AudisError> for UserFacingError {
    fn from(error: AudisError) -> Self {
        error.to_user_facing()
    }
}

impl serde::Serialize for AudisError {
    /// Errors returned from commands serialise as [`UserFacingError`], so an
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.to_user_facing().serialize(serializer)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn every_user_facing_error_states_whether_data_survived() {
        let errors = [
            AudisError::Configuration {
                detail: "bad toml".to_owned(),
            },
            AudisError::Io {
                path: PathBuf::from(r"C:\data"),
                detail: "denied".to_owned(),
                source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
            },
            AudisError::InvalidArgument {
                field: "sessionId".to_owned(),
                detail: "not a uuid".to_owned(),
            },
        ];

        for error in errors {
            let shown = UserFacingError::from(error);
            assert!(!shown.title.is_empty());
            assert!(!shown.explanation.is_empty());
            assert!(!shown.suggested_action.is_empty());
            assert!(shown.data_preserved);
        }
    }

    #[test]
    fn explanations_avoid_developer_jargon() {
        let shown = UserFacingError::from(AudisError::Io {
            path: PathBuf::from(r"C:\data\audis.db"),
            detail: "denied".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        });

        for banned in ["panic", "unwrap", "Err(", "backtrace", "thread '"] {
            assert!(
                !shown.explanation.contains(banned),
                "explanation leaked jargon: {banned}"
            );
        }
    }

    #[test]
    fn diagnostic_codes_are_stable_and_prefixed() {
        assert_eq!(
            DiagnosticCode::ConfigInvalid.as_str(),
            "AUDIS-CONFIG-INVALID"
        );
        for code in [
            DiagnosticCode::ConfigInvalid,
            DiagnosticCode::StorageUnavailable,
            DiagnosticCode::DataSerialization,
            DiagnosticCode::InvalidRequest,
            DiagnosticCode::Unexpected,
        ] {
            assert!(code.as_str().starts_with("AUDIS-"));
        }
    }

    #[test]
    fn io_errors_serialise_as_storage_problems_not_config_problems() {
        let error = AudisError::Io {
            path: PathBuf::from(r"C:\data"),
            detail: "denied".to_owned(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };

        let json = serde_json::to_value(&error).expect("serialise");

        assert_eq!(json["diagnosticCode"], "STORAGE_UNAVAILABLE");
        assert_eq!(json["dataPreserved"], true);
    }
}
