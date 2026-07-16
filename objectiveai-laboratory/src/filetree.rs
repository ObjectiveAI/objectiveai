//! The per-laboratory file-tree pump: proxy the container MCP's
//! `/filetree` SSE up to every connected daemon, verbatim.
//!
//! One pump per STARTED laboratory (spawned by the host's lazy
//! `lab_server` init, aborted on delete). It opens the container's
//! `/filetree` SSE — always the whole container, rooted at `/` — and,
//! for every event — the connect-time snapshot and each live delta —
//! folds it into the host's per-lab materialized tree and broadcasts
//! it to every connected daemon as an unsolicited
//! `laboratory_filetree` [`HostNotification`], the same fan-out
//! create/delete use. Pure push: nothing polls, nothing is pulled; the
//! open stream is the liveness. Mounted host folders are absent from
//! this stream — the host's own [`crate::mount_watch`] watches them
//! natively and grafts them into the same materialized tree.
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

/// Proxy `{base_url}/filetree` (the whole container) to every
/// connected daemon until aborted (laboratory delete / host shutdown).
pub async fn pump(host: Arc<HostServer>, id: String, base_url: String) {
    loop {
        let request = reqwest::Client::new().get(format!("{base_url}/filetree"));
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
                    match event {
                        FileTreeEvent::Snapshot { children } => {
                            host.source_container_snapshot(&id, children).await;
                        }
                        event => host.source_container_delta(&id, event).await,
                    }
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
