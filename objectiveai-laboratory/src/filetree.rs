//! The per-laboratory file-tree pump: proxy the container MCP's
//! `/filetree` SSE up to every connected daemon, verbatim.
//!
//! One pump per STARTED laboratory (spawned by the host's lazy
//! `lab_server` init, aborted on delete). It opens the container's
//! `/filetree` SSE scoped to the laboratory's cwd (its workspace) and,
//! for every event — the connect-time snapshot and each live delta —
//! folds it into the host's per-lab materialized tree and broadcasts
//! it to every connected daemon as an unsolicited
//! `laboratory_filetree` [`HostNotification`], the same fan-out
//! create/delete use. Pure push: nothing polls, nothing is pulled; the
//! open stream is the liveness.
//!
//! The materialized tree exists so a LATE-attaching daemon starts
//! current: `attach_channel` sends a synthesized snapshot per watched
//! laboratory (under the same `attach_lock` the folds hold, so the
//! snapshot-vs-delta race is closed by construction).
//!
//! Reconnect-forever, 1s pause — the same discipline as the daemon
//! channels themselves: a container restart re-opens the stream, whose
//! fresh connect-time snapshot replaces every downstream tree
//! (self-healing); a stopped container just keeps the pump quietly
//! retrying until the host shuts down or the laboratory is deleted.

use std::sync::Arc;

use futures::StreamExt;
use objectiveai_sdk::laboratories::filetree::FileTreeEvent;
use reqwest_eventsource::{Event, RequestBuilderExt};

use crate::host::HostServer;

/// Proxy `{base_url}/filetree?path={path}` to every connected daemon
/// until aborted (laboratory delete / host shutdown). `path` is the
/// laboratory's cwd; `None` falls back to the endpoint's default (the
/// whole container).
pub async fn pump(
    host: Arc<HostServer>,
    id: String,
    base_url: String,
    path: Option<String>,
) {
    loop {
        let mut request = reqwest::Client::new().get(format!("{base_url}/filetree"));
        if let Some(path) = &path {
            request = request.query(&[("path", path)]);
        }
        let Ok(mut source) = request.eventsource() else {
            // CannotCloneRequestError — a static builder bug, never
            // transient; retrying would spin.
            return;
        };
        while let Some(event) = source.next().await {
            match event {
                Ok(Event::Open) => continue,
                Ok(Event::Message(message)) => {
                    // Forward-compat: skip frames this build can't
                    // parse rather than tearing the watch down.
                    let Ok(event) = serde_json::from_str::<FileTreeEvent>(&message.data)
                    else {
                        continue;
                    };
                    host.filetree_event(&id, event).await;
                }
                // Transport error: the EventSource retries internally
                // (its own policy paces reconnection); a fresh connect
                // replays the lab's snapshot, resyncing everything.
                Err(_) => continue,
            }
        }
        // Stream permanently closed (fatal error path) — recreate it.
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
