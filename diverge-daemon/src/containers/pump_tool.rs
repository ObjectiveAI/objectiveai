//! A tool container's activity, off the run to its end.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::agents::logs::server::response::{self as log, Item};
use diverge_sdk::provider::endpoints::containers::tools::run::client::execute::{Event, ExecuteStream};
use diverge_sdk::shared::error::Error;
use futures_util::StreamExt as _;

use super::{ToolRun, activate, pump, stop};
use crate::daemon::Daemon;

/// Read the run's stream to its end: the proxy's `Active`, which
/// brackets the exchanges in flight there, is a use of the tool — the
/// daemon's own count of what is in flight is what holds the run up;
/// its `Inactive`, with what the program made of its rotating
/// secrets, written back and the run activated again while anything
/// is in flight into it, as [`inactive_tool`](super::inactive_tool)
/// states; and the stream's end or error the run over —
/// [`ended_tool`](stop::ended_tool), which releases what it holds.
/// A tool has no log of its own: an inactive that failed is kept on
/// the log of the agent a dependency tool is deployed for, naming the
/// tool's template and never a value, and said nowhere for a tool on
/// record.
pub async fn pump_tool(daemon: Arc<Daemon>, run: Arc<ToolRun>, mut stream: ExecuteStream) {
    while let Some(event) = stream.next().await {
        match event {
            Ok(Event::Active) => run.touch(),
            Ok(Event::Inactive(values)) => {
                run.touch();
                if let Err(error) = activate::inactive_tool(&daemon, &run, values).await {
                    noted(&daemon, &run, error).await;
                }
            }
            Err(_) => break,
        }
    }
    stop::ended_tool(&daemon, run).await;
}

/// The error kept on the owner agent's log, for a dependency tool
/// with one.
async fn noted(daemon: &Daemon, run: &ToolRun, error: Error) {
    let Some(tool) = &run.dependency else {
        return;
    };
    let Some(agent) = tool.owner_agent else {
        return;
    };
    let _ = pump::append_for(
        daemon,
        agent,
        Item::Error(log::Error {
            r#type: Default::default(),
            error: Error(serde_json::json!({
                "kind": "dependency",
                "dependency": tool.template,
                "error": error.0,
            })),
        }),
    )
    .await;
}
