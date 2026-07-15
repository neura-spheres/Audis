//! Audis application entry point.

mod audio_test;
mod commands;
mod credentials;
mod data_files;
mod logging;
mod model_store;
mod session;
mod settings_store;
mod transcript_store;
mod tray;

use audis_common::AppPaths;
use tauri::{Manager, WindowEvent};

use commands::AppState;
use settings_store::SettingsStore;

/// Refuse to open a window that could only show a browser error page.
///
/// A binary from plain `cargo build`/`cargo run` has no frontend inside it: it
/// loads the UI from `build.devUrl`. With no Vite server there, the window is
/// nothing but "127.0.0.1 refused to connect" while the log cheerfully reports
/// "Audis ready" — accurate and thoroughly misleading.
///
/// This is not hypothetical. `cargo build` and `tauri build --debug` write the
/// *same* path, `target/debug/audis-desktop.exe`, so a routine compile check
/// silently replaces a working app with one that cannot render, and the next
/// launch of a previously-fine exe shows the error page. Refusing to start is
/// what makes that impossible rather than merely documented: this binary is a
/// compile artifact, not an app, and it now says so instead of pretending.
///
/// Keyed on `tauri::is_dev()` rather than `debug_assertions`, because
/// `tauri build --debug` is also a debug binary but embeds the frontend and
/// works perfectly. Never fires for a shipped app.
fn exit_if_the_ui_cannot_load(app: &tauri::AppHandle) {
    if !tauri::is_dev() {
        return;
    }

    let Some(dev_url) = app.config().build.dev_url.clone() else {
        return;
    };

    if dev_server_is_reachable(&dev_url) {
        return;
    }

    let message = format!(
        "This binary has no user interface inside it: it expects a Vite dev server at {dev_url}, \
         and nothing is listening there.\n\n\
         It was produced by `cargo build` or `cargo run`, which are compile checks rather than \
         ways to run Audis. Opening a window now would only show a browser connection error.\n\n\
         To run Audis:\n  ./scripts/dev.ps1     (development, with hot reload)\n  \
         ./scripts/build.ps1   (a standalone app in target/release)"
    );

    tracing::error!(%dev_url, "refusing to start: this build has no UI and no dev server is running");
    eprintln!("\nAudis cannot start.\n\n{message}\n");

    // Before the window is shown, so the error page never reaches the screen.
    std::process::exit(2);
}

/// True when something is listening on the dev server's host and port.
///
/// A plain TCP connect: enough to tell "Vite is running" from "nothing is
/// there", without waiting on an HTTP response. Under `tauri dev` the CLI has
/// already waited for the server, so this is reached only when it is genuinely
/// absent — but it retries briefly rather than trusting one attempt.
fn dev_server_is_reachable(dev_url: &tauri::Url) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};

    let Some(host) = dev_url.host_str() else {
        return false;
    };
    let port = dev_url.port_or_known_default().unwrap_or(80);

    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        let Ok(addresses) = (host, port).to_socket_addrs() else {
            continue;
        };

        for address in addresses {
            if TcpStream::connect_timeout(&address, std::time::Duration::from_millis(500)).is_ok() {
                return true;
            }
        }
    }

    false
}

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
        .manage(session::SessionController::default())
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
            commands::start_session,
            commands::stop_session,
            commands::set_session_paused,
            commands::get_session_status,
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
            // Before the window is shown, so a UI-less build exits rather than
            // displaying a browser connection error.
            exit_if_the_ui_cannot_load(app.handle());

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
