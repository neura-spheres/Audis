//! User settings.
//!
//! Versioned so a future release can migrate an old file instead of discarding
//! it. Every field has a default, so a settings file that is partly corrupt
//! still loads with the readable parts intact.

use serde::{Deserialize, Serialize};

/// Schema version of the settings file.
pub const SETTINGS_VERSION: u32 = 1;

/// Which view Audis opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartPage {
    /// Open on the dashboard.
    #[default]
    Dashboard,
    /// Open on the session library.
    Sessions,
}

/// Appearance preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    /// Always light.
    Light,
    /// Always dark.
    Dark,
    /// Follow Windows.
    #[default]
    System,
}

/// What closing the main window does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CloseBehavior {
    /// Keep running in the notification area.
    #[default]
    MinimizeToTray,
    /// Quit Audis.
    Quit,
}

/// Settings that apply across the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GeneralSettings {
    /// Appearance.
    pub theme: ThemePreference,
    /// View shown on launch.
    pub start_page: StartPage,
    /// What the window close button does.
    pub close_behavior: CloseBehavior,
    /// Keep the tray icon visible while Audis runs.
    pub show_tray_icon: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::default(),
            start_page: StartPage::default(),
            close_behavior: CloseBehavior::default(),
            show_tray_icon: true,
        }
    }
}

/// Which devices to capture from.
///
/// `None` means "whatever Windows calls the default", which is what most people
/// want and what survives plugging in a headset mid-session.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AudioSettings {
    /// Microphone endpoint id, or the Windows default.
    pub microphone_id: Option<String>,
    /// Output endpoint to capture via loopback, or the Windows default.
    pub computer_audio_id: Option<String>,
}

/// What actually recognises speech.
///
/// The one setting in Audis that changes where your voice goes. Local keeps
/// audio on this PC and always works; a provider sends it over the internet in
/// exchange for accuracy no local model on a normal CPU can match. Because that
/// is a real trade rather than a preference, it is a single explicit choice
/// rather than something inferred from which model happens to be installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TranscriptionEngine {
    /// Whisper on this PC. Free, offline, private.
    Local {
        /// Which local model to load.
        model: crate::models::ModelId,
    },
    /// A cloud provider. Needs a key, an internet connection, and sends audio.
    Cloud {
        /// Which provider. Must be one where `can_transcribe` is true.
        provider: crate::providers::ProviderId,
        /// The provider's speech model.
        model: String,
    },
}

impl Default for TranscriptionEngine {
    fn default() -> Self {
        // Local by default, always. Audis' promise is that it works offline
        // with no account, and a default that quietly uploaded audio would
        // break that before the user had chosen anything.
        Self::Local {
            model: crate::models::ModelId::WhisperBase,
        }
    }
}

impl TranscriptionEngine {
    /// True when using this engine sends audio off this PC.
    pub fn sends_audio_away(&self) -> bool {
        matches!(self, Self::Cloud { .. })
    }
}

/// Speech recognition preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptionSettings {
    /// What recognises speech: a local model, or a cloud provider.
    pub engine: TranscriptionEngine,
    /// Which local model to use.
    ///
    /// Kept alongside `engine` so switching to a provider and back remembers
    /// the model you had, rather than silently resetting it.
    pub model: crate::models::ModelId,
    /// Which language to recognise.
    ///
    /// Always set, never detected: Audis supports exactly two languages, so
    /// telling the engine which one removes a failure mode rather than adding
    /// a setting.
    pub language: crate::language::Language,
    /// Capture the microphone.
    pub capture_microphone: bool,
    /// Capture what the computer is playing.
    pub capture_computer_audio: bool,
}

impl Default for TranscriptionSettings {
    fn default() -> Self {
        Self {
            engine: TranscriptionEngine::default(),
            model: crate::models::ModelId::WhisperBase,
            language: crate::language::Language::default(),
            capture_microphone: true,
            capture_computer_audio: true,
        }
    }
}

/// Caption appearance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptionSettings {
    /// Font size in pixels.
    pub font_size: u32,
    /// Lines kept on screen.
    pub max_lines: u32,
    /// Background opacity, 0 to 100.
    pub background_opacity: u32,
    /// Show which source each line came from.
    pub show_source_labels: bool,
    /// Let clicks pass through to whatever is behind.
    pub click_through: bool,
}

impl Default for CaptionSettings {
    fn default() -> Self {
        Self {
            font_size: 28,
            max_lines: 3,
            background_opacity: 70,
            show_source_labels: true,
            click_through: false,
        }
    }
}

/// Global shortcuts, as accelerator strings such as `CmdOrCtrl+Shift+A`.
///
/// `None` means unbound. Every one is optional: a shortcut that collides with
/// something the user needs is worse than no shortcut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShortcutSettings {
    /// Stop the running session.
    pub stop_session: Option<String>,
    /// Pause or resume.
    pub toggle_pause: Option<String>,
    /// Show or hide captions.
    pub toggle_captions: Option<String>,
    /// Ask the assistant.
    pub ask_assistant: Option<String>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            stop_session: Some("CmdOrCtrl+Shift+S".to_owned()),
            toggle_pause: Some("CmdOrCtrl+Shift+P".to_owned()),
            toggle_captions: Some("CmdOrCtrl+Shift+C".to_owned()),
            ask_assistant: Some("CmdOrCtrl+Shift+A".to_owned()),
        }
    }
}

/// Root settings object.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Schema version, used for migration.
    pub version: u32,
    /// Cross-cutting preferences.
    pub general: GeneralSettings,
    /// Capture device choices.
    pub audio: AudioSettings,
    /// Speech recognition.
    pub transcription: TranscriptionSettings,
    /// Caption appearance.
    pub captions: CaptionSettings,
    /// Global shortcuts.
    pub shortcuts: ShortcutSettings,
    /// Configured AI providers.
    ///
    /// Holds credential references, never keys. See `providers::ProviderConfig`.
    pub providers: Vec<crate::providers::ProviderConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            general: GeneralSettings::default(),
            audio: AudioSettings::default(),
            transcription: TranscriptionSettings::default(),
            captions: CaptionSettings::default(),
            shortcuts: ShortcutSettings::default(),
            providers: Vec::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn defaults_round_trip_through_json() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).expect("serialise");
        let parsed: Settings = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(settings, parsed);
    }

    /// Settings must never carry a secret: they are written to a plain JSON
    /// file that gets copied into support bundles.
    #[test]
    fn settings_never_serialise_an_api_key() {
        let mut settings = Settings::default();
        settings.providers.push(crate::providers::ProviderConfig {
            id: crate::providers::ProviderId::Gemini,
            enabled: true,
            model: "gemini-2.0-flash".to_owned(),
            endpoint: None,
            credential_ref: crate::providers::ProviderId::Gemini.credential_ref(),
        });

        let json = serde_json::to_string(&settings).expect("serialise");

        assert!(
            json.contains("provider/gemini/default"),
            "the reference must be stored"
        );
        for banned in ["apiKey", "api_key", "secret", "password"] {
            assert!(!json.contains(banned), "settings leaked a {banned} field");
        }
    }

    /// Audis' promise is that it works offline with no account. A default that
    /// uploaded audio would break that before the user chose anything, so the
    /// default engine is local and this test is what keeps it that way.
    #[test]
    fn speech_never_leaves_this_pc_by_default() {
        let settings = Settings::default();

        assert!(
            !settings.transcription.engine.sends_audio_away(),
            "the default engine must not send audio off this PC"
        );
        assert!(matches!(
            settings.transcription.engine,
            TranscriptionEngine::Local { .. }
        ));
    }

    /// A cloud engine must be honest about what it does.
    #[test]
    fn a_cloud_engine_declares_that_it_sends_audio_away() {
        let engine = TranscriptionEngine::Cloud {
            provider: crate::providers::ProviderId::Groq,
            model: "whisper-large-v3".to_owned(),
        };

        assert!(engine.sends_audio_away());
    }

    /// Only providers that can actually transcribe may be chosen as an engine.
    #[test]
    fn every_provider_offered_for_speech_can_actually_transcribe() {
        use crate::providers::ProviderId;

        assert!(ProviderId::Groq.can_transcribe());
        assert!(ProviderId::Gemini.can_transcribe());
        assert!(ProviderId::OpenAiCompatible.can_transcribe());
        // Neither has an audio API at all.
        assert!(!ProviderId::Anthropic.can_transcribe());
        assert!(!ProviderId::DeepSeek.can_transcribe());
    }

    #[test]
    fn transcription_defaults_to_the_recommended_free_model() {
        let settings = Settings::default();
        assert_eq!(
            settings.transcription.model,
            crate::models::ModelId::WhisperBase
        );
        assert!(settings.transcription.capture_microphone);
        assert!(settings.transcription.capture_computer_audio);
    }

    /// A settings file written by an older build will be missing fields added
    /// later. Those must fall back to defaults rather than failing the load and
    /// resetting everything the user configured.
    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed: Settings = serde_json::from_str("{}").expect("empty object must load");
        assert_eq!(parsed, Settings::default());

        let partial: Settings =
            serde_json::from_str(r#"{"general":{"theme":"dark"}}"#).expect("partial must load");
        assert_eq!(partial.general.theme, ThemePreference::Dark);
        // Untouched fields keep their defaults.
        assert_eq!(
            partial.general.close_behavior,
            CloseBehavior::MinimizeToTray
        );
        assert!(partial.general.show_tray_icon);
    }

    #[test]
    fn enums_serialise_as_camel_case_for_the_frontend() {
        let json = serde_json::to_value(Settings::default()).expect("serialise");
        assert_eq!(json["general"]["theme"], "system");
        assert_eq!(json["general"]["closeBehavior"], "minimizeToTray");
        assert_eq!(json["general"]["startPage"], "dashboard");
    }

    #[test]
    fn default_version_matches_the_constant() {
        assert_eq!(Settings::default().version, SETTINGS_VERSION);
    }
}
