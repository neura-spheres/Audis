//! Audio errors and how they read to a user.

use audis_common::{DiagnosticCode, UserFacingError};

use crate::device::DeviceKind;

/// Result alias for audio operations.
pub type Result<T, E = AudioError> = std::result::Result<T, E>;

/// What can go wrong capturing audio.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    /// The device list could not be read.
    #[error("could not list audio devices: {detail}")]
    Enumeration {
        /// Detail from the platform.
        detail: String,
    },

    /// No device of this kind exists.
    #[error("no {kind:?} device is available")]
    NoDevice {
        /// Which kind was wanted.
        kind: DeviceKind,
    },

    /// The device exists but its format could not be read.
    #[error("could not read the format of {device}: {detail}")]
    Format {
        /// Device name.
        device: String,
        /// Detail from the platform.
        detail: String,
    },

    /// The stream could not be opened.
    #[error("could not start capture on {device}: {detail}")]
    StreamStart {
        /// Device name.
        device: String,
        /// Detail from the platform.
        detail: String,
    },
}

impl AudioError {
    /// Translate into the message a user should read.
    pub fn to_user_facing(&self) -> UserFacingError {
        let technical_details = Some(self.to_string());

        match self {
            AudioError::Enumeration { .. } => UserFacingError {
                title: "Audis could not find your audio devices".to_owned(),
                explanation: "Windows did not return a list of audio devices. No session was \
                              started and nothing was recorded."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Check that Windows audio is working, then try again.".to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },

            AudioError::NoDevice { kind } => {
                let (title, explanation, action) = match kind {
                    DeviceKind::Input => (
                        "No microphone was found",
                        "Audis could not find a microphone to listen to. Nothing was recorded.",
                        "Connect a microphone, then refresh the device list.",
                    ),
                    DeviceKind::Output => (
                        "No audio output was found",
                        "Audis could not find speakers or headphones to capture from. Nothing \
                         was recorded.",
                        "Check your playback device in Windows sound settings, then try again.",
                    ),
                };
                UserFacingError {
                    title: title.to_owned(),
                    explanation: explanation.to_owned(),
                    data_preserved: true,
                    suggested_action: action.to_owned(),
                    technical_details,
                    diagnostic_code: DiagnosticCode::Unexpected,
                }
            }

            AudioError::Format { .. } => UserFacingError {
                title: "That device reported a format Audis cannot use".to_owned(),
                explanation: "Audis could not work out how this device sends audio, so it did \
                              not start. Nothing was recorded."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Try another device, or check its settings in Windows."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },

            AudioError::StreamStart { .. } => UserFacingError {
                title: "Audis could not start listening".to_owned(),
                explanation: "The audio device could not be opened. It may be in use by another \
                              application, or Windows may have denied access. Nothing was recorded."
                    .to_owned(),
                data_preserved: true,
                suggested_action: "Close other apps using the device, check that microphone \
                                   access is allowed in Windows privacy settings, then try again."
                    .to_owned(),
                technical_details,
                diagnostic_code: DiagnosticCode::Unexpected,
            },
        }
    }
}

impl From<AudioError> for UserFacingError {
    fn from(error: AudioError) -> Self {
        error.to_user_facing()
    }
}

impl serde::Serialize for AudioError {
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
    fn no_audio_error_ever_claims_data_was_lost() {
        let errors = [
            AudioError::Enumeration {
                detail: "x".to_owned(),
            },
            AudioError::NoDevice {
                kind: DeviceKind::Input,
            },
            AudioError::NoDevice {
                kind: DeviceKind::Output,
            },
            AudioError::Format {
                device: "Mic".to_owned(),
                detail: "x".to_owned(),
            },
            AudioError::StreamStart {
                device: "Mic".to_owned(),
                detail: "x".to_owned(),
            },
        ];

        for error in errors {
            let shown = error.to_user_facing();
            assert!(shown.data_preserved);
            assert!(!shown.title.is_empty());
            assert!(!shown.suggested_action.is_empty());
        }
    }

    /// A denied microphone is the most common real failure, so its advice must
    #[test]
    fn a_blocked_device_tells_the_user_where_to_look() {
        let shown = AudioError::StreamStart {
            device: "Microphone".to_owned(),
            detail: "access denied".to_owned(),
        }
        .to_user_facing();

        assert!(shown.suggested_action.to_lowercase().contains("privacy"));
    }
}
