//! Diverge — the desktop app.
//!
//! Rust owns the daemon, its address and its credentials; the page gets
//! commands (see [`actions`]) and one channel per stream. Built with the
//! `stand-in` feature (the default for now), the daemon and rooms are
//! stand-ins; without it, nothing answers yet and the screens say so. See
//! `diverge-desktop/CLAUDE.md`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Without the stand-in nothing reaches the agent door or most daemon verbs
// yet: on the wire, the daemon will.
#![cfg_attr(not(feature = "stand-in"), allow(dead_code))]

mod absent;
mod account;
mod actions;
mod catalog;
mod daemon;
mod machines;
mod marks;
mod door;
mod hires;
mod identity;
mod preview;
mod reporter;
mod spaces;
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
            let home = MenuItemBuilder::with_id("go-home", "Home").accelerator("CmdOrCtrl+1").build(app)?;
            let inbox = MenuItemBuilder::with_id("go-inbox", "Inbox").accelerator("CmdOrCtrl+2").build(app)?;
            let go = SubmenuBuilder::new(app, "Go").item(&home).item(&inbox).separator().item(&new_agent).separator().item(&close_tab).build()?;
            let window = SubmenuBuilder::new(app, "Window").minimize().maximize().separator().fullscreen().build()?;
            let menu = MenuBuilder::new(app).items(&[&app_menu, &edit, &go, &window]).build()?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                let _ = app.emit("menu://action", event.id().as_ref().to_owned());
            });

            // Every file the app keeps is in one folder: DIVERGE_DATA_DIR's, when it names one.
            let data = actions::data_dir(std::env::var_os(actions::DATA_DIR_VAR), || app.path().app_data_dir())?;
            let state = tauri::async_runtime::block_on(AppState::open(data));
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            actions::agents_list,
            actions::agents_create,
            actions::agents_delete,
            actions::agents_edit,
            actions::agents_mounts,
            actions::agents_message,
            actions::agents_message_take_back,
            actions::logs_open,
            actions::scope_close,
            actions::volumes_list,
            actions::volumes_stat,
            actions::volumes_tree,
            actions::volumes_read,
            actions::volumes_write,
            actions::volumes_room,
            actions::volumes_create,
            actions::volumes_room_for,
            actions::volumes_edit,
            actions::volumes_delete,
            actions::home_feed,
            actions::people_list,
            actions::profile_get,
            actions::spaces_home,
            actions::spaces_list,
            actions::spaces_get,
            actions::spaces_feed,
            actions::spaces_call,
            actions::spaces_watch,
            actions::spaces_host,
            actions::spaces_join,
            actions::spaces_leave,
            actions::spaces_invite,
            actions::knocks_watch,
            actions::knocks_answer,
            actions::spaces_door,
            actions::asks_send,
            actions::spaces_doorways,
            actions::vouch_for,
            actions::identity_broken,
            actions::asks_close,
            actions::spaces_admitted,
            actions::spaces_restart,
            actions::spaces_continue,
            actions::table_tree,
            actions::table_read,
            actions::table_write,
            actions::table_transfer,
            actions::personas_list,
            actions::persona_rename,
            actions::allowance_get,
            actions::allowance_set,
            actions::cards_watch,
            actions::cards_answer,
            actions::door_tools,
            actions::machines_list,
            actions::machines_add,
            actions::machines_remove,
            actions::machines_rename,
            actions::machines_names,
            actions::views_list,
            actions::views_save,
            actions::views_delete,
            actions::catalog_images,
            actions::catalog_check,
            actions::tabs_snapshot,
            actions::tabs_open,
            actions::tabs_close,
            actions::tabs_focus,
            actions::actions_list,
            actions::app_info,
            actions::files_set_aside,
            actions::first_run_get,
            actions::first_run_finish,
        ])
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
