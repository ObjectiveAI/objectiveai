//! Diverge — the desktop app, draft one.
//!
//! Rust owns the daemon, its address and its credentials; the page gets
//! commands (see [`actions`]) and one channel per stream. Today the daemon
//! is [`daemon::stub::StubDaemon`]; see `diverge-desktop/CLAUDE.md`.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod catalog;
mod daemon;
mod tabs;
mod view;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

use tauri::Manager;

use actions::AppState;
use daemon::stub::StubDaemon;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data)?;
            let host = data.join("stand-in-host");
            let daemon = tauri::async_runtime::block_on(async { StubDaemon::new(host) });
            let views_file = data.join("views.json");
            let views = std::fs::read_to_string(&views_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
            app.manage(AppState {
                stand_in_host: Some(daemon.host_root()),
                daemon: Arc::new(daemon),
                scopes: Mutex::new(HashMap::new()),
                next_scope: AtomicU64::new(1),
                tabs: Mutex::new(tabs::Tabs::default()),
                views: Mutex::new(views),
                views_file,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            actions::agents_list,
            actions::agents_create,
            actions::agents_delete,
            actions::agents_message,
            actions::agents_message_take_back,
            actions::logs_open,
            actions::scope_close,
            actions::files_tree_open,
            actions::files_read,
            actions::files_write,
            actions::machines_list,
            actions::machines_add,
            actions::machines_remove,
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
        ])
        .run(tauri::generate_context!())
        .expect("the app could not start");
}
