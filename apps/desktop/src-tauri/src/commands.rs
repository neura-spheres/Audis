//! Tauri commands exposed to the frontend.

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

    apply_caption_click_through(&app, saved.captions.click_through);

    app.emit(audis_common::events::SETTINGS_CHANGED, &saved)
        .ok();

    crate::shortcuts::apply(&app);

    Ok(saved)
}

/// Let clicks pass through the caption overlay to whatever is behind it.
pub fn apply_caption_click_through(app: &tauri::AppHandle, click_through: bool) {
    if let Some(window) = app.get_webview_window("captions")
        && let Err(error) = window.set_ignore_cursor_events(click_through)
    {
        tracing::warn!(%error, "could not change caption click-through");
    }
}

/// How far above the bottom of the screen the captions sit, in logical pixels.
const CAPTION_BOTTOM_MARGIN: f64 = 64.0;

/// Recentre the captions along the bottom and let them follow the content again.
#[tauri::command]
pub fn reset_caption_position(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("captions")
        && let (Ok(size), Ok(Some(monitor))) = (window.outer_size(), window.primary_monitor())
    {
        let scale = monitor.scale_factor();
        let screen_w = f64::from(monitor.size().width) / scale;
        let screen_h = f64::from(monitor.size().height) / scale;
        let win_w = f64::from(size.width) / scale;
        let win_h = f64::from(size.height) / scale;
        let x = ((screen_w - win_w) / 2.0).max(0.0);
        let y = (screen_h - win_h - CAPTION_BOTTOM_MARGIN).max(0.0);
        window.set_position(tauri::LogicalPosition::new(x, y)).ok();
    }

    Ok(())
}

/// Turn caption click-through on or off from anywhere, and apply it at once.
#[tauri::command]
pub fn set_caption_click_through(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    click_through: bool,
) -> Result<()> {
    let mut settings = state.settings.get();
    settings.captions.click_through = click_through;
    let saved = state.settings.set(settings)?;
    apply_caption_click_through(&app, click_through);
    app.emit(audis_common::events::SETTINGS_CHANGED, &saved)
        .ok();
    Ok(())
}

/// Look for a newer version of Audis on the user's chosen channel.
#[tauri::command]
pub async fn check_for_updates(app: tauri::AppHandle) -> Result<crate::updates::UpdateCheck> {
    let channel = app.state::<AppState>().settings.get().updates.channel;
    let result = crate::updates::check(channel, env!("CARGO_PKG_VERSION")).await?;

    app.emit(audis_common::events::UPDATE_STATUS, &result).ok();

    Ok(result)
}

/// Download and install the newest release on the user's channel, then restart.
///
/// The release is looked up again rather than taken from the caller, so what
/// gets installed is decided here against the saved channel, not by whatever the
/// UI happened to be showing.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<()> {
    let channel = app.state::<AppState>().settings.get().updates.channel;
    let found = crate::updates::check(channel, env!("CARGO_PKG_VERSION")).await?;

    let Some(release) = found.update else {
        tracing::info!("nothing to install: already up to date");
        return Ok(());
    };

    let Some(manifest) = release.manifest_url else {
        return Err(audis_common::AudisError::Configuration {
            detail: format!(
                "Version {} has no installer Audis can verify, so it cannot install it for you. \
                 Use Download to install it yourself.",
                release.version
            ),
        });
    };

    crate::updates::install(&app, &manifest).await
}

/// Open a release page in the browser so the user can install it themselves.
///
/// The URL is checked against the Audis repository rather than trusted: it
/// arrives from a network response, and opening whatever that response happened
/// to contain would hand a link of someone else's choosing to the browser.
#[tauri::command]
pub fn open_release_page(app: tauri::AppHandle, url: String) -> Result<()> {
    const RELEASES_PREFIX: &str = "https://github.com/neura-spheres/Audis/releases/";

    if !url.starts_with(RELEASES_PREFIX) {
        return Err(audis_common::AudisError::InvalidArgument {
            field: "url".to_owned(),
            detail: "that is not an Audis release page".to_owned(),
        });
    }

    app.opener().open_url(url, None::<&str>).map_err(|error| {
        audis_common::AudisError::Configuration {
            detail: format!("the release page could not be opened: {error}"),
        }
    })
}

/// Every file Audis has written, grouped by category.
#[tauri::command]
pub fn list_data_files(state: State<'_, AppState>) -> Result<DataFileListing> {
    data_files::list(&state.paths)
}

/// Open a file with its default application.
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
    Ok(models.list(&state.paths, state.settings.get().transcription.language))
}

/// Download and install a model, reporting progress on `audis://model/progress`.
#[tauri::command]
pub async fn install_model(app: tauri::AppHandle, id: audis_common::ModelId) -> Result<()> {
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
#[tauri::command]
pub fn set_provider_key(id: audis_common::ProviderId, key: String) -> Result<()> {
    tracing::info!(provider = ?id, "saving provider API key");
    crate::credentials::set_key(id, &key)
}

/// Delete a provider's API key.
#[tauri::command]
pub fn delete_provider_key(id: audis_common::ProviderId) -> Result<()> {
    tracing::info!(provider = ?id, "deleting provider API key");
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

    crate::overlays::hide_all(&app);

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

/// Hide one floating overlay, without ending the session.
#[tauri::command]
pub fn hide_overlay(app: tauri::AppHandle, overlay: String) -> Result<()> {
    let which = match overlay.as_str() {
        "captions" => crate::overlays::Overlay::Captions,
        "controller" => crate::overlays::Overlay::Controller,
        "assistant" => crate::overlays::Overlay::Assistant,
        other => {
            return Err(audis_common::AudisError::InvalidArgument {
                field: "overlay".to_owned(),
                detail: format!("unknown overlay {other:?}"),
            });
        }
    };

    crate::overlays::hide(&app, which);
    Ok(())
}

/// Bring the main window to the front, showing it if it was hidden.
#[tauri::command]
pub fn open_main_window(
    app: tauri::AppHandle,
    session: State<'_, crate::session::SessionController>,
) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }

    if session.status().is_some() {
        crate::overlays::show(&app, crate::overlays::Overlay::Captions);
        crate::overlays::show(&app, crate::overlays::Overlay::Controller);
        if app.state::<AppState>().settings.get().assistant.enabled {
            crate::overlays::place(&app, crate::overlays::Overlay::Assistant);
        }
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
            assistant_enabled: settings.assistant.enabled,
        },
    )
}

/// Build whatever the user chose to recognise speech with.
fn build_engine(
    state: &AppState,
    models: &std::sync::Arc<crate::model_store::ModelStore>,
    settings: &Settings,
) -> Result<Box<dyn audis_asr::AsrEngine>> {
    use audis_common::TranscriptionEngine;

    match &settings.transcription.engine {
        TranscriptionEngine::Local { model } => {
            tracing::info!(?model, "building local speech engine");
            let path = models.path_if_installed(&state.paths, *model).ok_or(
                audis_common::AudisError::Configuration {
                    detail: "no speech model is installed. Open Models and install Whisper Base."
                        .to_owned(),
                },
            )?;

            let engine = audis_asr::WhisperEngine::load(&path).map_err(as_configuration)?;
            Ok(Box::new(engine))
        }

        TranscriptionEngine::Cloud { provider, model } => {
            tracing::info!(?provider, %model, "building cloud speech engine");
            let key = crate::credentials::get_key(*provider)?.ok_or(
                audis_common::AudisError::Configuration {
                    detail: format!(
                        "Audis is set to transcribe with {}, but no API key is saved for it. \
                         Open Providers to add one.",
                        provider.info().name
                    ),
                },
            )?;

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

/// Every saved session, newest first.
#[tauri::command]
pub fn list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::transcript_store::SessionSummary>> {
    Ok(crate::transcript_store::list_summaries(&state.paths))
}

/// Every segment of one saved session.
#[tauri::command]
pub fn get_session_transcript(
    state: State<'_, AppState>,
    id: uuid::Uuid,
) -> Result<Vec<audis_common::TranscriptSegment>> {
    crate::transcript_store::read_segments(&state.paths, id)
}

/// Delete a saved session.
#[tauri::command]
pub fn delete_session(state: State<'_, AppState>, id: uuid::Uuid) -> Result<()> {
    tracing::info!(%id, "deleting session");
    crate::transcript_store::delete(&state.paths, id)
}

/// Export a session's transcript and reveal the file.
#[tauri::command]
pub fn export_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: uuid::Uuid,
    format: crate::transcript_store::ExportFormat,
) -> Result<String> {
    tracing::info!(%id, ?format, "exporting session transcript");
    let path = crate::transcript_store::export(&state.paths, id, format)?;
    app.opener().reveal_item_in_dir(&path).ok();
    Ok(path.display().to_string())
}

/// Answer a question with the assistant, using the transcript as context.
///
/// Returns an empty string when the assistant decides the line was not a real
/// question, so the caller can simply show nothing.
#[tauri::command]
pub async fn ask_assistant(
    app: tauri::AppHandle,
    question: String,
    transcript: Vec<String>,
    summary: String,
) -> Result<String> {
    let connection = assistant_connection(&app)?;
    let system = {
        let assistant = app.state::<AppState>().settings.get().assistant;
        assistant_system_prompt(assistant.context, &assistant.notes)
    };
    let AssistantConnection {
        provider,
        model,
        endpoint,
        key,
    } = connection;

    let mut user = String::new();
    if !summary.trim().is_empty() {
        user.push_str(&format!(
            "Summary of the session so far:\n{}\n\n",
            summary.trim()
        ));
    }
    user.push_str(&format!(
        "Transcript around the question (older lines first):\n{}\n\nThe question to answer:\n{question}",
        transcript.join("\n")
    ));

    tracing::info!(?provider, %model, "asking the assistant");

    // Broadcast so the assistant overlay and the main window both react, even
    // though only one of them made the call.
    app.emit(
        audis_common::events::ASSISTANT_STATUS,
        serde_json::json!({ "thinking": true }),
    )
    .ok();

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        audis_asr::chat(provider, endpoint, key, &model, &system, &user)
    })
    .await
    .map_err(|error| audis_common::AudisError::Configuration {
        detail: format!("the assistant request could not run: {error}"),
    })?
    .map(|answer| {
        // The prompt tells the model to reply NONE when the line was not a real
        // question; turn that into an empty answer the UI simply hides.
        if answer.trim().eq_ignore_ascii_case("none") {
            String::new()
        } else {
            answer.trim().to_owned()
        }
    });

    app.emit(
        audis_common::events::ASSISTANT_STATUS,
        serde_json::json!({ "thinking": false }),
    )
    .ok();

    let answer = outcome.map_err(as_configuration)?;

    if !answer.is_empty() {
        app.emit(
            audis_common::events::ASSISTANT_RESPONSE,
            serde_json::json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "question": question,
                "answer": answer,
            }),
        )
        .ok();
    }

    Ok(answer)
}

/// Turn the assistant on or off from anywhere, including the controller chip.
#[tauri::command]
pub fn set_assistant_enabled(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<()> {
    let mut settings = state.settings.get();
    settings.assistant.enabled = enabled;
    let saved = state.settings.set(settings)?;
    app.emit(audis_common::events::SETTINGS_CHANGED, &saved)
        .ok();

    // During a session, place the panel beside the chip when turning on so it is
    // ready to appear; hide it outright when turning off. The panel itself keeps
    // its visibility in step with the setting via the settings-changed event.
    let running = app
        .state::<crate::session::SessionController>()
        .status()
        .is_some();
    if running {
        if enabled {
            crate::overlays::place(&app, crate::overlays::Overlay::Assistant);
        } else {
            crate::overlays::hide(&app, crate::overlays::Overlay::Assistant);
        }
    }

    tracing::info!(enabled, "assistant toggled");
    Ok(())
}

/// Everything needed to reach the assistant's chat provider.
struct AssistantConnection {
    provider: audis_common::ProviderId,
    model: String,
    endpoint: Option<String>,
    key: String,
}

/// Read the assistant's provider, model, endpoint and key from settings.
fn assistant_connection(app: &tauri::AppHandle) -> Result<AssistantConnection> {
    let settings = app.state::<AppState>().settings.get();
    let assistant = settings.assistant;

    let key = crate::credentials::get_key(assistant.provider)?.ok_or(
        audis_common::AudisError::Configuration {
            detail: format!(
                "The assistant is set to use {}, but no API key is saved for it. Open Providers \
                 to add one.",
                assistant.provider.info().name
            ),
        },
    )?;

    let endpoint = settings
        .providers
        .iter()
        .find(|config| config.id == assistant.provider)
        .and_then(|config| config.endpoint.clone());

    Ok(AssistantConnection {
        provider: assistant.provider,
        model: assistant.model,
        endpoint,
        key,
    })
}

/// Fold new transcript lines into a compact running summary of the session.
///
/// This is how the assistant keeps track of a whole call without the cost of
/// sending the entire transcript with every question: the frontend hands over a
/// batch of older lines every so often and gets back an updated summary.
#[tauri::command]
pub async fn assistant_summarize(
    app: tauri::AppHandle,
    previous: String,
    lines: Vec<String>,
) -> Result<String> {
    if lines.is_empty() {
        return Ok(previous);
    }

    let AssistantConnection {
        provider,
        model,
        endpoint,
        key,
    } = assistant_connection(&app)?;

    let context = app.state::<AppState>().settings.get().assistant.context;
    let system = assistant_summary_prompt(context);

    let user = format!(
        "Summary so far (may be empty):\n{}\n\nNew lines to fold in:\n{}",
        previous.trim(),
        lines.join("\n")
    );

    tracing::info!(?provider, %model, lines = lines.len(), "updating the session summary");

    let summary = tauri::async_runtime::spawn_blocking(move || {
        audis_asr::chat(provider, endpoint, key, &model, &system, &user)
    })
    .await
    .map_err(|error| audis_common::AudisError::Configuration {
        detail: format!("the summary request could not run: {error}"),
    })?
    .map_err(as_configuration)?;

    Ok(summary.trim().to_owned())
}

fn assistant_summary_prompt(context: audis_common::AssistantContext) -> String {
    use audis_common::AssistantContext::*;

    let kind = match context {
        General => "conversation",
        Meeting => "meeting",
        Interview => "interview",
        Quiz => "quiz or test",
        Lecture => "lecture",
    };

    format!(
        "You keep a running summary of a live {kind}. Given the summary so far and new transcript \
         lines, return an updated summary that stays under 150 words. Keep the concrete facts, \
         names, numbers, decisions, and open questions; drop small talk and filler. Write it as \
         plain notes, not a narrative. Output only the summary."
    )
}

fn assistant_system_prompt(context: audis_common::AssistantContext, notes: &str) -> String {
    use audis_common::AssistantContext::*;

    let role = match context {
        General => {
            "You help someone during a live conversation. When a question is asked, answer it \
             briefly and accurately."
        }
        Meeting => {
            "You assist during a live meeting. When a question comes up, give a concise, factual \
             answer or the relevant information."
        }
        Interview => {
            "You are helping the user, who is the candidate in a live job interview. When the \
             interviewer asks a question, suggest a strong, concise, well-structured answer the \
             user could give in first person."
        }
        Quiz => {
            "You are helping the user during a live quiz or test. Give the correct answer \
             concisely, with a one-line reason."
        }
        Lecture => {
            "You assist a student during a live lecture. Answer questions and clarify concepts \
             briefly."
        }
    };

    let mut prompt = format!(
        "{role}\n\nYou are given the recent transcript of the session. Answer the latest question \
         in at most three sentences. If the latest line is not actually a question that needs an \
         answer, reply with exactly: NONE"
    );

    if !notes.trim().is_empty() {
        prompt.push_str(&format!(
            "\n\nExtra context about this session: {}",
            notes.trim()
        ));
    }

    prompt
}

/// What a fetched model list is for, as the frontend names it.
#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelPurpose {
    /// Models that transcribe audio.
    Speech,
    /// Models that hold a conversation.
    Chat,
}

impl From<ModelPurpose> for audis_asr::ModelPurpose {
    fn from(purpose: ModelPurpose) -> Self {
        match purpose {
            ModelPurpose::Speech => Self::Speech,
            ModelPurpose::Chat => Self::Chat,
        }
    }
}

/// Ask a provider for its current model list, filtered to `purpose`.
#[tauri::command]
pub async fn list_provider_models(
    app: tauri::AppHandle,
    provider: audis_common::ProviderId,
    purpose: ModelPurpose,
) -> Result<Vec<String>> {
    let (endpoint, key) = {
        let state = app.state::<AppState>();
        let endpoint = state
            .settings
            .get()
            .providers
            .iter()
            .find(|config| config.id == provider)
            .and_then(|config| config.endpoint.clone());
        let key = crate::credentials::get_key(provider)?.ok_or(
            audis_common::AudisError::Configuration {
                detail: format!("no API key saved for {}", provider.info().name),
            },
        )?;
        (endpoint, key)
    };

    tauri::async_runtime::spawn_blocking(move || {
        audis_asr::fetch_models(provider, endpoint, key, purpose.into())
    })
    .await
    .map_err(|error| audis_common::AudisError::Configuration {
        detail: format!("could not fetch the model list: {error}"),
    })?
    .map_err(as_configuration)
}
