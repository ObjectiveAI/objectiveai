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

use diverge_sdk::keyring::Keyring;
use tauri::Manager;

use actions::AppState;

/// The app: no menu of its own, so the window has no menu bar; the
/// page handles every shortcut. On macOS the system's default menu
/// stays in the system's bar, which is what gives text fields their
/// Edit keys there.
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The root, as the daemon and the provider find theirs; the
            // document in it; the key it names, from the root's
            // keyring; the app's own folder under the root. An entry
            // the keyring does not hold stops the start, naming it.
            let (root, config, key) = tauri::async_runtime::block_on(async {
                let root = diverge_viewer_config::root(std::env::args_os().skip(1)).await?;
                let config = diverge_viewer_config::load(&root).await?;
                let keyring = Keyring::open(&root).await?;
                let key = keyring
                    .get(&config.daemon.key)
                    .await?
                    .ok_or_else(|| format!("the keyring holds no `{}`, the key viewer.yaml names", config.daemon.key))?;
                Ok::<_, Box<dyn std::error::Error>>((root, config, key))
            })?;
            // The daemon's connection, held for the page: the SDK's runner.
            app.manage(diverge_sdk::tauri_proxy::TauriProxy::new(config.daemon.address, &key));
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
