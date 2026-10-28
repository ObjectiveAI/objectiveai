//! Diverge World.
//!
//! Empty up-front. Rust holds the daemon's address, the identity it is
//! dialled as, and the one connection to it; the page speaks the
//! daemon's protocol through the TypeScript SDK over the SDK's
//! `tauri_proxy` runner, and holds nothing else. See
//! `diverge-world/CLAUDE.md`.
//!
//! The root — `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge`
//! — is the one directory every Diverge program on the host shares, and
//! the app reads the viewer's document in it: `<root>/viewer.yaml` says
//! which daemon, as which of `<root>/identities/`, and how the daemon is
//! dialled again when the connection ends (`diverge_viewer_config`). The
//! app keeps no files.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use diverge_sdk::identities::{Identities, Name, Presenter};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // The root, as the daemon and the provider find theirs; the
            // document in it, which must name the daemon and the
            // identity it is dialled as; the root's identities. A
            // document naming no daemon stops the start, saying so.
            let (daemon, identities) = tauri::async_runtime::block_on(async {
                let root = diverge_viewer_config::root(std::env::args_os().skip(1)).await?;
                let config = diverge_viewer_config::load(&root).await?;
                let daemon = config.daemon.ok_or(
                    "viewer.yaml names no daemon identity (daemon: {address, issuer, subject}); sign one in with `diverge identities login`",
                )?;
                let identities = Identities::open(&root).await?;
                Ok::<_, Box<dyn std::error::Error>>((daemon, identities))
            })?;
            // The daemon's connection, held for the page: the SDK's
            // runner, presenting the identity, taking the daemon's
            // answer at its word, and dialling again as viewer.yaml's
            // `redial` says.
            let name = Name { issuer: daemon.issuer.clone(), subject: daemon.subject };
            let presenter = Arc::new(Presenter::new(identities, name));
            app.manage(diverge_sdk::tauri_proxy::TauriProxy::new(daemon.address, daemon.issuer, presenter, daemon.redial));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
