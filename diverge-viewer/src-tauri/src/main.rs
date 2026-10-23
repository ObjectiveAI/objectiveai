//! Diverge — the viewer.
//!
//! Rust owns the daemon's address and credential and the one connection
//! to it; the page speaks the daemon's protocol through the TypeScript
//! SDK, frame by frame over the proxy commands, and asks Rust for the
//! app's own things — tabs, the theme, saved Views, the names it gave
//! machines — through [`actions`]. See `diverge-viewer/CLAUDE.md`.

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

            // Every file the app keeps is in one folder: DIVERGE_DATA_DIR's, when it names one.
            let data = actions::data_dir(std::env::var_os(actions::DATA_DIR_VAR), || app.path().app_data_dir())?;
            app.manage(AppState::open(data));
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
        ])
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
