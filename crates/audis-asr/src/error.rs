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

    /// A cloud provider could not be reached.
    #[error("could not reach {provider}: {detail}")]
    ProviderUnreachable {
        /// Provider name, for the message.
        provider: String,
        /// Detail from the HTTP client.
        detail: String,
    },

    /// A cloud provider refused the request.
    #[error("{provider} refused the request ({status}): {detail}")]
    ProviderRejected {
        /// Provider name, for the message.
        provider: String,
        /// HTTP status.
        status: u16,
        /// The provider's own message, when it gave one.
        detail: String,
    },

    /// A provider is configured for speech but has no key saved.
    #[error("no API key saved for {provider}")]
    ProviderKeyMissing {
        /// Provider name, for the message.
        provider: String,
    },
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

            AsrError::ProviderUnreachable { provider, .. } => UserFacingError {
                title: format!("Audis could not reach {provider}"),
                explanation: "Cloud transcription needs an internet connection, and this request                               did not get through. Nothing you said was lost from the recording                               on this PC."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Check your internet connection. To keep working offline, open                                    Transcription and switch back to a model that runs on this PC."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },

            AsrError::ProviderRejected {
                provider, status, ..
            } => UserFacingError {
                title: match status {
                    402 => format!("{provider} has run out of credit"),
                    429 => format!("{provider} is rate limiting Audis"),
                    _ => format!("{provider} refused the request"),
                },
                explanation: match status {
                    401 | 403 => format!(
                        "{provider} did not accept your API key. It may be wrong, revoked, or \
                         lack permission for speech."
                    ),
                    402 => format!(
                        "Your {provider} account has no credit left, so it is refusing every \
                         request. Nothing you said was transcribed."
                    ),
                    429 => format!(
                        "You have hit {provider}'s rate limit, or run out of free quota for now."
                    ),
                    _ => format!("{provider} returned an error and could not transcribe the audio."),
                },
                data_preserved: true,
                suggested_action: match status {
                    401 | 403 => "Open Providers and save the key again.".to_owned(),
                    402 => format!(
                        "Add credit to your {provider} account, or pick another speech engine in \
                         Transcription. Running Whisper on this PC is free."
                    ),
                    429 => "Wait a little, or switch to a model that runs on this PC in \
                            Transcription."
                        .to_owned(),
                    _ => "Try again. If it continues, switch to a model that runs on this PC."
                        .to_owned(),
                },
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },

            AsrError::ProviderKeyMissing { provider } => UserFacingError {
                title: format!("No API key for {provider}"),
                explanation: format!(
                    "Audis is set to transcribe with {provider}, but no key is saved for it."
                ),
                data_preserved: true,
                suggested_action: "Open Providers and save a key, or switch to a model that runs                                    on this PC in Transcription."
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

    fn rejected(status: u16) -> AsrError {
        AsrError::ProviderRejected {
            provider: "Deepgram".to_owned(),
            status,
            detail: r#"{"err_code":"ASR_PAYMENT_REQUIRED"}"#.to_owned(),
        }
    }

    #[test]
    fn running_out_of_credit_says_so_and_offers_a_way_out() {
        // A real 402 from Deepgram once read as "returned an error", which told
        // the user nothing they could act on.
        let facing = rejected(402).to_user_facing();

        assert!(facing.title.contains("run out of credit"));
        assert!(facing.explanation.contains("no credit left"));
        // The way out has to be reachable without spending money.
        assert!(facing.suggested_action.contains("Transcription"));
        assert!(facing.suggested_action.contains("free"));
    }

    #[test]
    fn a_rejected_key_and_a_rate_limit_do_not_read_the_same() {
        let key = rejected(401).to_user_facing();
        let limit = rejected(429).to_user_facing();
        let credit = rejected(402).to_user_facing();

        assert!(key.explanation.contains("API key"));
        assert!(limit.explanation.contains("rate limit"));

        // Three different faults with three different fixes: if any two read the
        // same, the message is not doing its job.
        assert_ne!(key.suggested_action, limit.suggested_action);
        assert_ne!(limit.suggested_action, credit.suggested_action);
        assert_ne!(key.suggested_action, credit.suggested_action);
    }

    #[test]
    fn a_provider_message_never_arrives_with_the_gaps_a_broken_wrap_leaves() {
        for status in [401, 402, 429, 500] {
            let facing = rejected(status).to_user_facing();
            for text in [&facing.title, &facing.explanation, &facing.suggested_action] {
                assert!(
                    !text.contains("  "),
                    "{status} reads with a run of spaces in it: {text:?}"
                );
            }
        }
    }

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
