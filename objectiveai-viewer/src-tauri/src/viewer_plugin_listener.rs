//! The viewer plugin listener — the second daemon `/listen` client,
//! feeding PLUGIN destinations.
//!
//! Its OWN connection to the daemon broadcast (the daemon feeds every
//! connected socket a full copy), fully independent of the main
//! passthrough in [`crate::daemon_ws`]: no shared state, and neither
//! client's backpressure or failures touch the other. It watches the
//! requests that feed into it and ignores everything except
//! `plugins/run` runs (direct leaf, or a transform-bearing request
//! whose inner request is `plugins/run`). Each such run is
//! re-packaged into the same standard three-frame envelope the main
//! passthrough emits, destined to the TARGET PLUGIN's coordinates
//! (the `owner`/`name`/`version` named by the request itself):
//! `Event::Inbound { destination: Plugin{…}, value: <frame> }`.
//!
//! Combined with the main passthrough, a `plugins/run` run is
//! double-fed to the JS side — once to the main viewer UI, once to
//! the plugin it targets. The main viewer's bridge routes the
//! plugin-destined copy into the matching iframe; all routing lives
//! host-side, none in the JS SDK.
//!
//! Skipped runs are simply dropped — their typed envelopes (and the
//! nested response streams) fall out of scope here, which detaches
//! the feeds inside the SDK listener's pump. Nothing accumulates.

use futures::StreamExt;
use objectiveai_sdk::cli::command::{ListenerExecution, Request, plugins};
use objectiveai_sdk::viewer::{Destination, EventSender};

/// Spawn the resident client task. Best-effort forever-loop, exactly
/// like the main passthrough's: any failure falls through to a short
/// sleep and a fresh attempt against the same address. Exits only
/// when the event bus receiver is gone (the viewer is shutting down).
pub(crate) fn spawn_client(tx: EventSender, address: String, signature: Option<String>) {
    tokio::spawn(async move {
        loop {
            if pump(&tx, &address, signature.as_deref()).await.is_err() {
                // Receiver gone: the viewer is shutting down.
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

/// One connection: run the typed listener until its stream ends,
/// re-packaging every `plugins/run` run for its target plugin and
/// dropping everything else. `Err(())` means the event bus is closed —
/// stop entirely.
async fn pump(tx: &EventSender, url: &str, signature: Option<&str>) -> Result<(), ()> {
    let Ok(mut listener) = crate::daemon_ws::connect(url, signature).await else {
        return Ok(());
    };
    while let Some(item) = listener.next().await {
        let Ok(execution) = item else {
            // Transport error: the listener's stream ends right after —
            // fall through to the reconnect loop.
            break;
        };
        let Some(destination) = plugin_destination(&execution) else {
            continue;
        };
        crate::daemon_ws::emit_run(tx, execution, destination)?;
    }
    Ok(())
}

/// The plugin a run feeds: a `plugins/run` request — direct leaf or
/// transform-bearing — names its target plugin. Every other run is
/// `None` (dropped by this client).
fn plugin_destination(execution: &ListenerExecution) -> Option<Destination> {
    let request = match execution {
        ListenerExecution::Plugins(plugins::ListenerExecution::Run(run)) => &run.request,
        ListenerExecution::Transformed { request, .. } => match request.as_ref() {
            Request::Plugins(plugins::Request::Run(request)) => request,
            _ => return None,
        },
        _ => return None,
    };
    Some(Destination::Plugin {
        owner: request.owner.clone(),
        name: request.name.clone(),
        version: request.version.clone(),
    })
}
