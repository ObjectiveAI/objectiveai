//! The daemon → viewer stdin channel (`--features stdio` builds only).
//!
//! The viewer twin of the laboratory host's stdin loop
//! (`objectiveai-laboratory/src/main.rs::stdin_loop`), with its
//! discipline: one JSON object per line, unparseable lines ignored,
//! the ack printed AFTER the command is applied, and stdin EOF as the
//! daemon's graceful-shutdown signal.
//!
//! ONE deviation, and it is the reason this file is feature-gated at
//! all: EOF only means "exit" once the channel has proven itself —
//! i.e. after the FIRST successfully parsed frame. The daemon seeds
//! the registration list immediately after the ready handshake, so a
//! daemon-owned stdin always parses a frame before it can EOF. A
//! stray launch of a stdio-built binary (double-clicked exe, null
//! stdin, instant EOF, zero frames) logs and disarms instead of
//! killing a viewer the daemon never owned.

use tauri::Manager;

/// Spawn the resident stdin reader. Called once from setup.
pub fn spawn_dev_stdin(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tokio::io::AsyncBufReadExt;
        let mut armed = false;
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let Some(request) =
                        objectiveai_sdk::viewer_stdio::parse_viewer_stdio_request(
                            &line,
                        )
                    else {
                        continue;
                    };
                    match request.command {
                        objectiveai_sdk::viewer_stdio::ViewerStdioCommand::SetDevelopmentPlugins {
                            plugins,
                        } => {
                            apply(&app, plugins).await;
                        }
                    }
                    armed = true;
                    // Ack AFTER applying — application, not receipt.
                    objectiveai_sdk::viewer_stdio::print_viewer_stdio_ack(
                        &objectiveai_sdk::viewer_stdio::ViewerStdioAck {
                            id: request.id,
                        },
                    );
                }
                Ok(None) | Err(_) => {
                    if armed {
                        // The daemon closed our stdin: the graceful
                        // half of the stdio-child contract.
                        app.exit(0);
                    } else {
                        super::report_shell(
                            &app,
                            "info",
                            "dev: stdin closed before any frame — not \
                             daemon-owned, stdio channel disarmed"
                                .to_string(),
                        )
                        .await;
                    }
                    return;
                }
            }
        }
    });
}

/// Converge onto a new full registration list: swap the registry,
/// re-arm the watcher, refresh the inventory (dev plugins' tabs appear
/// and disappear here), and reload the open tabs of every trio whose
/// registration CHANGED — their resolution changed under them.
async fn apply(
    app: &tauri::AppHandle,
    plugins: Vec<objectiveai_sdk::viewer_stdio::DevelopmentViewerPlugin>,
) {
    let dev = app.state::<super::DevPlugins>();
    let changed = dev.set(plugins.into_iter().map(|plugin| {
        (
            super::dev::dev_key(&plugin.owner, &plugin.name, &plugin.version),
            std::path::PathBuf::from(plugin.path),
        )
    }));

    if let Some(tx) = app.try_state::<super::devwatch::DevWatchTx>() {
        let _ = tx.0.send(super::devwatch::DevWatchMsg::Rearm);
    }

    let model = app.state::<super::ShellModel>();
    let windows = model.windows_full().await;
    let plugins_root = app.state::<super::PluginsDirs>().plugins_root();
    if let Some(window) = windows.keys().next().cloned() {
        super::rescan_and_apply(app, &plugins_root, &window, true).await;
    }

    // A changed registration means every open tab of that trio is
    // rendering from the WRONG source now — reload them onto the new
    // resolution (dev → live dir, deleted → installed copy).
    for (owner, name, version) in changed {
        let identity = format!("{owner}/{name}/{version}");
        for state in windows.values() {
            for tab in &state.tabs {
                if tab.kind.identity == identity {
                    let label = super::native::tab_label(tab.id);
                    if let Some(webview) = app.get_webview(&label) {
                        let _ = webview.reload();
                    }
                }
            }
        }
    }
}
