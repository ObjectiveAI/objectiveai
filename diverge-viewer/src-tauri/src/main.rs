//! Diverge — the viewer.
//!
//! Rust owns the daemon's address and credential and the one connection
//! to it; the page speaks the daemon's protocol through the TypeScript
//! SDK, frame by frame over the proxy commands, and asks Rust for the
//! app's own things — tabs, the theme, saved Views, the names it gave
//! machines — through [`actions`]. See `diverge-viewer/CLAUDE.md`.
//!
//! The root — `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge`
//! — is the one directory every Diverge program on the host shares:
//! `<root>/viewer.yaml` says which daemon and as whom
//! (`diverge_viewer_config`), and `<root>/viewer/` holds every file the
//! app keeps.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod store;
mod tabs;
mod view;

use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::{Emitter, Manager};

use actions::AppState;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The menu: the OS's own Edit items (text fields need them), and
            // the app's shortcuts. ⌘W closes a tab, not the window.
            let app_menu = SubmenuBuilder::new(app, "Diverge").about(None).separator().services().separator().hide().hide_others().show_all().separator().quit().build()?;
            let edit = SubmenuBuilder::new(app, "Edit").undo().redo().separator().cut().copy().paste().select_all().build()?;
            let new_agent = MenuItemBuilder::with_id("new-agent", "New Agent").accelerator("CmdOrCtrl+N").build(app)?;
            let close_tab = MenuItemBuilder::with_id("close-tab", "Close Tab").accelerator("CmdOrCtrl+W").build(app)?;
            let settings = MenuItemBuilder::with_id("go-settings", "Settings").accelerator("CmdOrCtrl+,").build(app)?;
            let go = SubmenuBuilder::new(app, "Go").item(&new_agent).separator().item(&settings).separator().item(&close_tab).build()?;
            let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().fullscreen().build()?;
            let menu = MenuBuilder::new(app).items(&[&app_menu, &edit, &go, &window]).build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu://action", event.id().as_ref().to_owned());
            });

            // The root, as the daemon and the provider find theirs; the
            // document in it; the app's own folder under it.
            let (root, config) = tauri::async_runtime::block_on(async {
                let root = diverge_viewer_config::root(std::env::args_os().skip(1)).await?;
                let config = diverge_viewer_config::load(&root).await?;
                Ok::<_, diverge_viewer_config::Error>((root, config))
            })?;
            // The daemon's connection, held for the page: the SDK's runner.
            app.manage(diverge_sdk::tauri_proxy::TauriProxy::new(config.daemon.address, config.daemon.key));
            app.manage(AppState::open(root.join(actions::FOLDER)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            actions::machines_names,
            actions::machines_rename,
            actions::theme_get,
            actions::theme_set,
            actions::views_list,
            actions::views_save,
            actions::views_delete,
            actions::tabs_snapshot,
            actions::tabs_open,
            actions::tabs_close,
            actions::tabs_focus,
            actions::actions_list,
            actions::app_info,
            actions::files_set_aside,
            diverge_sdk::tauri_proxy::wire_request,
            diverge_sdk::tauri_proxy::wire_channel_request,
            diverge_sdk::tauri_proxy::wire_channel_response,
            diverge_sdk::tauri_proxy::wire_channel_response_finish,
            diverge_sdk::tauri_proxy::wire_status,
            diverge_sdk::tauri_proxy::wire_reset,
        ])
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
