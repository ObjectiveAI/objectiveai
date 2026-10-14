//! Every service brought up at the daemon's start.

use std::sync::Arc;

use diverge_sdk::daemon::endpoints::standalone_tools::templates::Mode;

use super::{held, keep};
use crate::containers::ToolKey;
use crate::daemon::Daemon;
use crate::store::{self, dependency_tools, tools};

/// At the daemon's start: every service on record with an agent
/// attached is kept — its supervisor starts its container — a
/// dependency tool whose template is a service, and a tool on record
/// whose template's mode is. One with none attached is left as it is:
/// nothing holds it, and the next attach or connect that holds it
/// keeps it up.
pub async fn boot(daemon: &Arc<Daemon>) -> Result<(), store::Error> {
    let kept = {
        let mut conn = daemon.store.acquire().await?;
        let mut kept = Vec::new();
        for tool in dependency_tools::all(&mut conn).await? {
            if tool.is_service() && !dependency_tools::attachments::attached(&mut conn, tool.id).await?.is_empty() {
                kept.push(ToolKey::Dependency(tool.id));
            }
        }
        for tool in tools::all(&mut conn).await? {
            if tools::mode(&mut conn, &tool).await? == Some(Mode::Service) && !tools::attachments::of_tool(&mut conn, tool.id).await?.is_empty() {
                kept.push(ToolKey::Record(tool.id));
            }
        }
        kept
    };
    for key in kept {
        keep(daemon, key).await;
        held(daemon, key).await;
    }
    Ok(())
}
