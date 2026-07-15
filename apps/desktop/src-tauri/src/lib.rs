//! Audis application entry point.

mod audio_test;
mod commands;
mod credentials;
mod data_files;
mod logging;
mod model_store;
mod settings_store;
mod tray;

use audis_common::AppPaths;
use tauri::{Manager, WindowEvent};

use commands::AppState;
use settings_store::SettingsStore;

/// Build and run the application.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Resolve and create the data tree before anything tries to log into it.
    let paths = match AppPaths::discover().and_then(|paths| {
        paths.ensure_created()?;
        Ok(paths)
    }) {
        Ok(paths) => paths,
        Err(error) => {
            // Logging is not up yet, since its directory is what just failed.
            eprintln!("Audis could not prepare its data directory: {error}");
            std::process::exit(1);
        }
    };

    logging::init(&paths);

    let settings = SettingsStore::load(&paths);

    tauri::Builder::default()
        // Tauri requires the single-instance plugin to be registered first.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tracing::info!("second launch intercepted; focusing existing window");
            tray::focus_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .manage(AppState { paths, settings })
        .manage(audio_test::AudioTestState::default())
        .manage(std::sync::Arc::new(model_store::ModelStore::default()))
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_settings,
            commands::update_settings,
            commands::list_data_files,
            commands::open_data_file,
            commands::reveal_data_file,
            commands::get_diagnostics,
            commands::list_audio_devices,
            commands::start_audio_test,
            commands::stop_audio_test,
            commands::list_models,
            commands::install_model,
            commands::cancel_model_download,
            commands::is_model_downloading,
            commands::remove_model,
            commands::list_providers,
            commands::set_provider_key,
            commands::delete_provider_key,
            commands::update_provider,
            commands::list_features,
            commands::close_main_window,
        ])
        .on_window_event(|window, event| {
            // Closing the main window follows the user's preference rather than
            // always quitting, so a running session is not lost to a stray
            // click on the X.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() != "main" {
                    return;
                }
                let app = window.app_handle();
                let state = app.state::<AppState>();
                use audis_common::settings::CloseBehavior;
                if state.settings.get().general.close_behavior == CloseBehavior::MinimizeToTray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let state = app.state::<AppState>();
            if state.settings.get().general.show_tray_icon {
                tray::build(app.handle())?;
            }

            // The window starts hidden so it can be shown once the WebView has
            // content, which avoids a white flash on launch.
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
            }

            tracing::info!("Audis ready");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            tracing::error!(%error, "the Audis event loop failed to start");
            std::process::exit(1);
        });
}
