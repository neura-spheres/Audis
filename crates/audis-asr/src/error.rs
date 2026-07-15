//! Recognition errors and how they read to a user.

use audis_common::{DiagnosticCode, UserFacingError};

/// Result alias for recognition operations.
pub type Result<T, E = AsrError> = std::result::Result<T, E>;

/// What can go wrong recognising speech.
#[derive(Debug, thiserror::Error)]
pub enum AsrError {
    /// The model file is not on disk.
    #[error("no model at {path}")]
    ModelMissing {
        /// Where Audis looked.
        path: String,
    },

    /// The model exists but could not be loaded.
    #[error("could not load the model at {path}: {detail}")]
    ModelLoad {
        /// Where the model is.
        path: String,
        /// Detail from the engine.
        detail: String,
    },

    /// Recognition itself failed.
    #[error("recognition failed: {detail}")]
    Recognition {
        /// Detail from the engine.
        detail: String,
    },

    /// This build has no local engine compiled in.
    #[error("this build has no local speech engine")]
    NoLocalEngine,
}

impl AsrError {
    /// Translate into the message a user should read.
    pub fn to_user_facing(&self) -> UserFacingError {
        let technical_details = Some(self.to_string());

        match self {
            AsrError::ModelMissing { .. } => UserFacingError {
                title: "No speech model is installed".to_owned(),
                explanation: "Audis needs a speech model to turn audio into text, and none is \
                              installed yet. Nothing was recorded."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Open Models and install Whisper Base. It is free and runs on \
                                   this PC."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::ConfigInvalid,
            },

            AsrError::ModelLoad { .. } => UserFacingError {
                title: "The speech model could not be loaded".to_owned(),
                explanation: "The model file is there but could not be read. It may have been \
                              interrupted while downloading. Your sessions were not affected."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Open Models, remove the model, and install it again.".to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::DataSerialization,
            },

            AsrError::Recognition { .. } => UserFacingError {
                title: "Audis could not transcribe that audio".to_owned(),
                explanation: "The speech engine failed on a piece of audio. The session is still \
                              running and everything already transcribed was kept."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "If this keeps happening, try a smaller model in Models."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },

            AsrError::NoLocalEngine => UserFacingError {
                title: "This build has no local speech engine".to_owned(),
                explanation: "Audis was built without the on-device speech engine, so it cannot \
                              transcribe locally. Nothing was recorded."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Use an official Audis build, or rebuild with the local-whisper \
                                   feature enabled."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::ConfigInvalid,
            },
        }
    }
}

impl From<AsrError> for UserFacingError {
    fn from(error: AsrError) -> Self {
        error.to_user_facing()
    }
}

impl serde::Serialize for AsrError {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&self.to_user_facing(), serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_recognition_error_claims_data_was_lost() {
        let errors = [
            AsrError::ModelMissing {
                path: "x".to_owned(),
            },
            AsrError::ModelLoad {
                path: "x".to_owned(),
                detail: "y".to_owned(),
            },
            AsrError::Recognition {
                detail: "y".to_owned(),
            },
            AsrError::NoLocalEngine,
        ];

        for error in errors {
            let shown = error.to_user_facing();
            assert!(shown.data_preserved);
            assert!(!shown.suggested_action.is_empty());
        }
    }

    /// The most common first-run failure. Its advice must name the fix.
    #[test]
    fn a_missing_model_points_at_the_models_page() {
        let shown = AsrError::ModelMissing {
            path: "x".to_owned(),
        }
        .to_user_facing();

        assert!(shown.suggested_action.contains("Models"));
    }
}
