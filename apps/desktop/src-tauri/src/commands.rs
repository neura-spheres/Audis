//! Tauri commands exposed to the frontend.
//!
//! Commands return `Result<T, AudisError>`. `AudisError` serialises as
//! `UserFacingError`, so an internal message cannot reach the UI.

use audis_common::{AppInfo, AppPaths, DataFileListing, Result, Settings, identity};
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::data_files;
use crate::settings_store::SettingsStore;

/// Process-wide state shared by every command.
pub struct AppState {
    /// Resolved data directory layout.
    pub paths: AppPaths,
    /// User settings, backed by `<data>/settings.json`.
    pub settings: SettingsStore,
}

/// Identity and build information for the About page and diagnostics.
///
/// The version comes from the compiled binary rather than a frontend constant,
/// so the About page cannot drift out of date after a release.
#[tauri::command]
pub fn get_app_info(state: State<'_, AppState>) -> Result<AppInfo> {
    Ok(AppInfo {
        app_name: identity::APP_NAME.to_owned(),
        company: identity::COMPANY.to_owned(),
        publisher: identity::PUBLISHER.to_owned(),
        tagline: identity::TAGLINE.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        bundle_id: identity::BUNDLE_ID.to_owned(),
        data_dir: state.paths.root().display().to_string(),
    })
}

/// Current user settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    Ok(state.settings.get())
}

/// Replace user settings and persist them.
#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings> {
    let saved = state.settings.set(settings)?;

    // Click-through is a window property, not something the page can style, so
    // applying it is Rust's job. Without this the switch would save a value
    // that never took effect.
    apply_caption_click_through(&app, saved.captions.click_through);

    // Announced to every window, not just the one that made the change: the
    // caption overlay renders from these and would otherwise keep showing the
    // old size and opacity until Audis restarted.
    app.emit(audis_common::events::SETTINGS_CHANGED, &saved)
        .ok();

    Ok(saved)
}

/// Let clicks pass through the caption overlay to whatever is behind it.
///
/// Kept in one place because two callers need it: saving the setting, and
/// starting a session, which is when the overlay first appears.
pub fn apply_caption_click_through(app: &tauri::AppHandle, click_through: bool) {
    if let Some(window) = app.get_webview_window("captions")
        && let Err(error) = window.set_ignore_cursor_events(click_through)
    {
        tracing::warn!(%error, "could not change caption click-through");
    }
}

/// Every file Audis has written, grouped by category.
#[tauri::command]
pub fn list_data_files(state: State<'_, AppState>) -> Result<DataFileListing> {
    data_files::list(&state.paths)
}

/// Open a file with its default application.
///
/// `path` comes from the frontend and is therefore untrusted: it is resolved
/// and confined to the data folder before anything is opened.
#[tauri::command]
pub fn open_data_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<()> {
    let resolved = data_files::resolve_inside_root(&state.paths, &path)?;

    app.opener()
        .open_path(resolved.display().to_string(), None::<&str>)
        .map_err(|error| audis_common::AudisError::Io {
            path: resolved,
            detail: "could not open that file".to_owned(),
            source: std::io::Error::other(error.to_string()),
        })
}

/// Show a file in File Explorer with it selected.
#[tauri::command]
pub fn reveal_data_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<()> {
    let resolved = data_files::resolve_inside_root(&state.paths, &path)?;

    app.opener()
        .reveal_item_in_dir(&resolved)
        .map_err(|error| audis_common::AudisError::Io {
            path: resolved,
            detail: "could not show that file in File Explorer".to_owned(),
            source: std::io::Error::other(error.to_string()),
        })
}

/// Environment information for the diagnostics page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    /// Audis version.
    pub app_version: String,
    /// Host OS name and version.
    pub os: String,
    /// CPU architecture.
    pub arch: String,
    /// WebView2 runtime version, when it can be determined.
    pub webview_version: Option<String>,
    /// Tauri version this build links against.
    pub tauri_version: String,
    /// Data root.
    pub data_dir: String,
    /// Log directory.
    pub logs_dir: String,
    /// Total bytes across the data directory.
    pub storage_bytes: u64,
    /// Number of files in the data directory.
    pub file_count: usize,
}

/// Real environment information, gathered on demand.
#[tauri::command]
pub fn get_diagnostics(state: State<'_, AppState>) -> Result<Diagnostics> {
    // A failed listing should not blank the whole page; report zeroes instead.
    let listing = data_files::list(&state.paths).unwrap_or_else(|error| {
        tracing::warn!(%error, "could not measure storage for diagnostics");
        DataFileListing {
            root: state.paths.root().display().to_string(),
            groups: Vec::new(),
            total_bytes: 0,
            total_files: 0,
        }
    });

    Ok(Diagnostics {
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        os: format!("{} {}", std::env::consts::OS, os_version()),
        arch: std::env::consts::ARCH.to_owned(),
        webview_version: tauri::webview_version().ok(),
        tauri_version: tauri::VERSION.to_owned(),
        data_dir: state.paths.root().display().to_string(),
        logs_dir: state.paths.logs_dir().display().to_string(),
        storage_bytes: listing.total_bytes,
        file_count: listing.total_files,
    })
}

fn os_version() -> String {
    tauri_plugin_os::version().to_string()
}

/// Every microphone and output endpoint on this machine.
#[tauri::command]
pub fn list_audio_devices()
-> std::result::Result<audis_audio::AudioDevices, audis_audio::AudioError> {
    audis_audio::enumerate()
}

/// Start the audio test: open both captures and stream levels to the UI.
///
/// Returns which streams opened and why any failed. One source failing does not
/// prevent the other from running.
#[tauri::command]
pub fn start_audio_test(
    app: tauri::AppHandle,
    state: State<'_, crate::audio_test::AudioTestState>,
    microphone_id: Option<String>,
    computer_audio_id: Option<String>,
) -> Result<crate::audio_test::AudioTestStatus> {
    Ok(state.start(&app, microphone_id, computer_audio_id))
}

/// Stop the audio test and release both devices.
#[tauri::command]
pub fn stop_audio_test(state: State<'_, crate::audio_test::AudioTestState>) -> Result<()> {
    state.stop();
    Ok(())
}

/// Every speech model, with whether it is installed.
#[tauri::command]
pub fn list_models(
    state: State<'_, AppState>,
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
) -> Result<Vec<audis_common::InstalledModel>> {
    // The recommendation depends on the language being recognised: Base is
    // good at English and poor at Indonesian, so a single global "recommended"
    // badge would mislead exactly the users Audis is built for.
    Ok(models.list(&state.paths, state.settings.get().transcription.language))
}

/// Download and install a model, reporting progress on `audis://model/progress`.
#[tauri::command]
pub async fn install_model(app: tauri::AppHandle, id: audis_common::ModelId) -> Result<()> {
    // `State` guards are not `Send`, and this download runs for minutes, so
    // everything needed is cloned out and the guards dropped before awaiting.
    // The store is managed as an `Arc` precisely so this clone is cheap and
    // safe rather than requiring a raw pointer.
    let (paths, store) = {
        let state = app.state::<AppState>();
        let models = app.state::<std::sync::Arc<crate::model_store::ModelStore>>();
        (state.paths.clone(), std::sync::Arc::clone(&models))
    };

    store.install(app.clone(), paths, id).await
}

/// Stop the running model download.
#[tauri::command]
pub fn cancel_model_download(
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
) -> Result<()> {
    models.cancel();
    Ok(())
}

/// Whether a model download is currently running.
///
/// Lets the UI restore its progress state after a navigation, since a download
/// outlives the view that started it.
#[tauri::command]
pub fn is_model_downloading(
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
) -> Result<bool> {
    Ok(models.is_downloading())
}

/// Delete an installed model.
#[tauri::command]
pub fn remove_model(
    state: State<'_, AppState>,
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
    id: audis_common::ModelId,
) -> Result<()> {
    models.remove(&state.paths, id)
}

/// Every AI provider and whether a key is saved.
///
/// Reports only whether a key exists, never any part of its value.
#[tauri::command]
pub fn list_providers(state: State<'_, AppState>) -> Result<Vec<audis_common::ProviderStatus>> {
    let settings = state.settings.get();

    Ok(audis_common::ProviderId::ALL
        .iter()
        .map(|id| {
            let info = id.info();
            let saved = settings.providers.iter().find(|config| config.id == *id);

            audis_common::ProviderStatus {
                has_key: crate::credentials::has_key(*id),
                enabled: saved.is_some_and(|config| config.enabled),
                model: saved
                    .map(|config| config.model.clone())
                    .unwrap_or_else(|| info.default_model.clone()),
                endpoint: saved.and_then(|config| config.endpoint.clone()),
                info,
            }
        })
        .collect())
}

/// Save an API key to the OS credential store.
///
/// The key goes straight to the keystore and is never written to settings, a
/// log, or anywhere Audis controls. There is deliberately no command to read it
/// back.
#[tauri::command]
pub fn set_provider_key(id: audis_common::ProviderId, key: String) -> Result<()> {
    crate::credentials::set_key(id, &key)
}

/// Delete a provider's API key.
#[tauri::command]
pub fn delete_provider_key(id: audis_common::ProviderId) -> Result<()> {
    crate::credentials::delete_key(id)
}

/// Enable or configure a provider.
#[tauri::command]
pub fn update_provider(
    state: State<'_, AppState>,
    id: audis_common::ProviderId,
    enabled: bool,
    model: String,
    endpoint: Option<String>,
) -> Result<()> {
    let mut settings = state.settings.get();

    let config = audis_common::ProviderConfig {
        id,
        enabled,
        model,
        endpoint,
        credential_ref: id.credential_ref(),
    };

    match settings.providers.iter_mut().find(|saved| saved.id == id) {
        Some(existing) => *existing = config,
        None => settings.providers.push(config),
    }

    state.settings.set(settings)?;
    Ok(())
}

/// Every feature, with whether it can actually be started right now.
#[tauri::command]
pub fn list_features(
    state: State<'_, AppState>,
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
) -> Result<Vec<audis_common::Feature>> {
    use audis_common::{FeatureId, FeatureStatus};

    let settings = state.settings.get();
    let has_model = models
        .path_if_installed(&state.paths, settings.transcription.model)
        .is_some();
    let has_provider = audis_common::ProviderId::ALL
        .iter()
        .any(|id| crate::credentials::has_key(*id));

    Ok(FeatureId::ALL
        .iter()
        .map(|id| {
            let (name, summary, details) = id.describe();

            // Status is computed from what is actually on this machine rather
            // than hardcoded, so the launcher cannot claim a feature works when
            // its model or key is missing.
            let needs_ai = id.uses_cloud_ai();

            let (status, blocker) = if !has_model {
                (
                    FeatureStatus::NeedsSetup,
                    Some("Install a speech model first. Open Models and choose Whisper Base; it is free.".to_owned()),
                )
            } else if needs_ai && !has_provider {
                (
                    FeatureStatus::NeedsSetup,
                    Some("Connect an AI provider first. Open Providers; Gemini and Groq have free tiers.".to_owned()),
                )
            } else {
                (FeatureStatus::Ready, None)
            };

            audis_common::Feature {
                id: *id,
                name: name.to_owned(),
                summary: summary.to_owned(),
                details: details.iter().map(|line| (*line).to_owned()).collect(),
                status,
                blocker,
                uses_cloud: needs_ai,
            }
        })
        .collect())
}

/// Apply the window close behaviour the user chose.
#[tauri::command]
pub fn close_main_window(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<()> {
    use audis_common::settings::CloseBehavior;

    match state.settings.get().general.close_behavior {
        CloseBehavior::MinimizeToTray => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        CloseBehavior::Quit => app.exit(0),
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample_info() -> AppInfo {
        AppInfo {
            app_name: identity::APP_NAME.to_owned(),
            company: identity::COMPANY.to_owned(),
            publisher: identity::PUBLISHER.to_owned(),
            tagline: identity::TAGLINE.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            bundle_id: identity::BUNDLE_ID.to_owned(),
            data_dir: AppPaths::rooted_at(r"C:\data\Audis")
                .root()
                .display()
                .to_string(),
        }
    }

    #[test]
    fn app_info_reports_identity_and_a_real_version() {
        let info = sample_info();

        assert_eq!(info.app_name, "Audis");
        assert_eq!(info.company, "Neura Audis");
        assert_eq!(info.bundle_id, "ai.neura.audis");
        assert!(!info.version.is_empty());
    }

    /// The frontend schema expects camelCase, so the serde rename is
    /// load-bearing rather than cosmetic.
    #[test]
    fn app_info_serialises_as_camel_case_for_the_frontend() {
        let json = serde_json::to_value(sample_info()).expect("serialise AppInfo");

        assert!(json.get("appName").is_some());
        assert!(json.get("bundleId").is_some());
        assert!(json.get("dataDir").is_some());
        assert!(
            json.get("app_name").is_none(),
            "snake_case must not leak to the UI"
        );
    }

    #[test]
    fn diagnostics_serialise_as_camel_case() {
        let diagnostics = Diagnostics {
            app_version: "0.1.0".to_owned(),
            os: "windows 11".to_owned(),
            arch: "x86_64".to_owned(),
            webview_version: Some("120".to_owned()),
            tauri_version: "2".to_owned(),
            data_dir: r"C:\data".to_owned(),
            logs_dir: r"C:\data\logs".to_owned(),
            storage_bytes: 10,
            file_count: 2,
        };

        let json = serde_json::to_value(&diagnostics).expect("serialise");

        assert!(json.get("appVersion").is_some());
        assert!(json.get("webviewVersion").is_some());
        assert!(json.get("storageBytes").is_some());
    }
}

/// Start a live session for `feature`.
///
/// Resolves the model and devices from settings, so the UI does not have to
/// know what a session needs.
#[tauri::command]
pub fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    models: State<'_, std::sync::Arc<crate::model_store::ModelStore>>,
    session: State<'_, crate::session::SessionController>,
    feature: audis_common::FeatureId,
) -> Result<audis_common::SessionStatus> {
    let settings = state.settings.get();

    let engine = build_engine(&state, &models, &settings)?;

    session.start(
        &app,
        crate::session::SessionRequest {
            engine,
            paths: state.paths.clone(),
            mode: feature,
            language: settings.transcription.language,
            microphone_id: settings.audio.microphone_id.clone(),
            computer_audio_id: settings.audio.computer_audio_id.clone(),
            want_microphone: settings.transcription.capture_microphone,
            want_computer_audio: settings.transcription.capture_computer_audio,
        },
    )
}

/// Build whatever the user chose to recognise speech with.
///
/// The key is read here and handed to the engine directly, never through
/// settings and never back to the frontend: this is the one place in Audis that
/// holds a plaintext key, and it holds it only for the life of the session.
fn build_engine(
    state: &AppState,
    models: &std::sync::Arc<crate::model_store::ModelStore>,
    settings: &Settings,
) -> Result<Box<dyn audis_asr::AsrEngine>> {
    use audis_common::TranscriptionEngine;

    match &settings.transcription.engine {
        TranscriptionEngine::Local { model } => {
            let path = models.path_if_installed(&state.paths, *model).ok_or(
                audis_common::AudisError::Configuration {
                    detail: "no speech model is installed. Open Models and install Whisper Base."
                        .to_owned(),
                },
            )?;

            // Loading is seconds of work and allocates the whole model. Doing it
            // here means a missing or corrupt file fails before any device is
            // opened, rather than half-starting a session.
            let engine = audis_asr::WhisperEngine::load(&path).map_err(as_configuration)?;
            Ok(Box::new(engine))
        }

        TranscriptionEngine::Cloud { provider, model } => {
            let key = crate::credentials::get_key(*provider)?.ok_or(
                audis_common::AudisError::Configuration {
                    detail: format!(
                        "Audis is set to transcribe with {}, but no API key is saved for it.                          Open Providers to add one.",
                        provider.info().name
                    ),
                },
            )?;

            // A user-supplied endpoint only matters for providers that need one;
            // CloudEngine ignores it otherwise.
            let endpoint = settings
                .providers
                .iter()
                .find(|config| config.id == *provider)
                .and_then(|config| config.endpoint.clone());

            let engine = audis_asr::CloudEngine::new(*provider, model.clone(), endpoint, key)
                .map_err(as_configuration)?;
            Ok(Box::new(engine))
        }
    }
}

/// Carry an ASR failure out through a command.
///
/// `AsrError` already knows how to explain itself to a user, but a command
/// returns `AudisError`, which has no variant that can hold a ready-made
/// message. The explanation is passed through as the detail so the words the
/// user reads are still the specific ones — "could not reach Groq", not a
/// generic failure — even though the surrounding frame says configuration.
/// Giving `AudisError` a variant that wraps a `UserFacingError` would be the
/// better fix and is a wider change than this one.
fn as_configuration(error: audis_asr::AsrError) -> audis_common::AudisError {
    let facing = error.to_user_facing();
    audis_common::AudisError::Configuration {
        detail: format!("{} {}", facing.explanation, facing.suggested_action),
    }
}

/// Stop the running session and release every device.
#[tauri::command]
pub fn stop_session(
    app: tauri::AppHandle,
    session: State<'_, crate::session::SessionController>,
) -> Result<audis_common::SessionStatus> {
    session.stop(&app)
}

/// Pause or resume the running session.
#[tauri::command]
pub fn set_session_paused(
    app: tauri::AppHandle,
    session: State<'_, crate::session::SessionController>,
    paused: bool,
) -> Result<audis_common::SessionStatus> {
    session.set_paused(&app, paused)
}

/// The running session, if there is one.
#[tauri::command]
pub fn get_session_status(
    session: State<'_, crate::session::SessionController>,
) -> Result<Option<audis_common::SessionStatus>> {
    Ok(session.status())
}
