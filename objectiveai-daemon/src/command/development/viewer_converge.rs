//! Push the FULL viewer development-registration list to the viewer
//! over its stdin — the viewer twin of the laboratory dial-list
//! converge (`command/laboratories/spawn.rs::converge`), sharing its
//! properties:
//!
//! - Its OWN serialization gate, never nested inside another.
//! - The desired state is re-read FRESH under the gate, so the last
//!   converge in gate order sends a list at least as new as every
//!   registry write preceding its lock.
//! - No viewer, or a broken/deaf pipe ⇒ `Ok(None)`, not an error —
//!   write-only semantics. The registry survives independently, and
//!   the next viewer spawn seeds from it.
//!
//! ONE deviation: the ack wait is BOUNDED. The viewer's stdio handling
//! is a build-time feature (`--features stdio`); a binary built
//! without it never reads its pipe and never acks, and an unbounded
//! wait against one would hang `viewer spawn` forever. A deaf viewer
//! degrades to "dev registrations don't reach it", nothing worse.

use crate::context::GlobalContext;
use crate::error::Error;

/// How long a viewer gets to ack a converge before it is treated as
/// stdio-incapable. Generous next to the work involved (apply a list,
/// print a line) while still bounding `viewer spawn`.
const ACK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

/// `Ok(Some(()))` = the viewer acked the new list; `Ok(None)` = there
/// was nobody to tell (no viewer, no pipe, no ack) — valid, not an
/// error.
pub(crate) async fn viewer_converge(
    global: &GlobalContext,
) -> Result<Option<()>, Error> {
    let gate = global.spawn_gate("viewer/dev-converge");
    let _guard = gate.lock().await;
    // Viewer check FIRST — a viewerless converge has no side effects.
    let Some(stdio) = global.viewer_stdio() else {
        return Ok(None);
    };
    let Some(hubs) = global.resident_hubs() else {
        return Ok(None);
    };
    let plugins = hubs
        .development_plugins
        .viewer
        .list()
        .into_iter()
        .map(|((owner, name, version), path)| {
            objectiveai_sdk::viewer_stdio::DevelopmentViewerPlugin {
                owner,
                name,
                version,
                path: path.to_string_lossy().into_owned(),
            }
        })
        .collect();
    let command =
        objectiveai_sdk::viewer_stdio::ViewerStdioCommand::SetDevelopmentPlugins {
            plugins,
        };
    if stdio.send_stdio_bounded(&command, ACK_DEADLINE).await.is_err() {
        // Died mid-send, or built without the stdio feature. Either
        // way: nobody to tell.
        return Ok(None);
    }
    Ok(Some(()))
}
