//! System-tray icon and menu.

use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
};

/// Bring the main window to the foreground, restoring and focusing it.
///
/// Also used by the single-instance handler, so launching Audis twice focuses
/// the existing window instead of opening another one.
pub fn focus_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    } else {
        tracing::warn!("main window not found when trying to focus it");
    }
}

/// Build the tray icon and its menu.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Audis", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Audis", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::with_id("audis-tray")
        .icon(app.default_window_icon().cloned().ok_or_else(|| {
            tauri::Error::AssetNotFound("default window icon is missing".to_owned())
        })?)
        .tooltip("Audis")
        // Menu on right-click only, leaving left-click free to mean "open the
        // window", which is what Windows users expect.
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => focus_main_window(app),
            "quit" => {
                tracing::info!("quit requested from tray");
                app.exit(0);
            }
            other => tracing::warn!(menu_item = other, "unhandled tray menu item"),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { .. } = event {
                focus_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
