//! Reading the asks, and handing each to its task.

use std::sync::Arc;

use futures_util::StreamExt as _;

use super::super::family::Runs;
use super::super::run::Run;
use super::{command, fuse, mcp, postgres, vault};
use crate::container_proxy::requests::execute::{Ask, ExecuteStream};
use crate::container_proxy::requests::request::Request;

/// Carry every ask until there are no more, then say the container
/// is gone.
///
/// The stream ends cleanly when the proxy closes `/requests` — the
/// container stopping on its own — and with one error when the
/// socket died, which is the same fact told worse. Either way nothing
/// more will be asked, and a run with no container is over.
pub(crate) async fn relay<R: Runs>(run: Arc<Run>, mut asks: ExecuteStream) {
    while let Some(Ok(ask)) = asks.next().await {
        run.spawn(answer::<R>(Arc::clone(&run), ask)).await;
    }
    run.over.notify_one();
}

/// One ask, answered.
async fn answer<R: Runs>(run: Arc<Run>, ask: Ask) {
    let kind = match ask.frame() {
        Ok(frame) => Kind::of(&frame.request),
        // Validated once on arrival; unreachable, and nothing to
        // answer on a path this end cannot name.
        Err(_) => return,
    };
    match kind {
        Kind::McpListTools => mcp::list_tools::<R>(run, ask).await,
        Kind::McpListResources => mcp::list_resources::<R>(run, ask).await,
        Kind::McpCallTool => mcp::call_tool::<R>(run, ask).await,
        Kind::McpReadResource => mcp::read_resource::<R>(run, ask).await,
        Kind::McpNotifications => mcp::notifications::<R>(run, ask).await,
        Kind::VaultGet => vault::get::<R>(run, ask).await,
        Kind::VaultSet => vault::set::<R>(run, ask).await,
        Kind::VaultDelete => vault::delete::<R>(run, ask).await,
        Kind::VaultLock => vault::lock::<R>(run, ask).await,
        Kind::VaultUnlock => vault::unlock::<R>(run, ask).await,
        Kind::Command => command::command::<R>(run, ask).await,
        Kind::Postgres => postgres::postgres::<R>(run, ask).await,
        Kind::FuseRead => fuse::read::<R>(run, ask).await,
        Kind::FuseWrite => fuse::write::<R>(run, ask).await,
        Kind::FuseList => fuse::list::<R>(run, ask).await,
        Kind::FuseRemove => fuse::remove::<R>(run, ask).await,
        Kind::FuseRename => fuse::rename::<R>(run, ask).await,
        Kind::FuseMkdir => fuse::mkdir::<R>(run, ask).await,
        Kind::FuseStat => fuse::stat::<R>(run, ask).await,
    }
}

/// Which ask, without the borrow: the decoded request borrows the
/// ask's bytes, and the task takes the ask.
enum Kind {
    McpListTools,
    McpListResources,
    McpCallTool,
    McpReadResource,
    McpNotifications,
    VaultGet,
    VaultSet,
    VaultDelete,
    VaultLock,
    VaultUnlock,
    Command,
    Postgres,
    FuseRead,
    FuseWrite,
    FuseList,
    FuseRemove,
    FuseRename,
    FuseMkdir,
    FuseStat,
}

impl Kind {
    fn of(request: &Request<'_>) -> Self {
        match request {
            Request::McpListTools(_) => Kind::McpListTools,
            Request::McpListResources(_) => Kind::McpListResources,
            Request::McpCallTool(_) => Kind::McpCallTool,
            Request::McpReadResource(_) => Kind::McpReadResource,
            Request::McpNotifications(_) => Kind::McpNotifications,
            Request::VaultGet(_) => Kind::VaultGet,
            Request::VaultSet(_) => Kind::VaultSet,
            Request::VaultDelete(_) => Kind::VaultDelete,
            Request::VaultLock(_) => Kind::VaultLock,
            Request::VaultUnlock(_) => Kind::VaultUnlock,
            Request::Command(_) => Kind::Command,
            Request::Postgres(_) => Kind::Postgres,
            Request::FuseRead(_) => Kind::FuseRead,
            Request::FuseWrite(_) => Kind::FuseWrite,
            Request::FuseList(_) => Kind::FuseList,
            Request::FuseRemove(_) => Kind::FuseRemove,
            Request::FuseRename(_) => Kind::FuseRename,
            Request::FuseMkdir(_) => Kind::FuseMkdir,
            Request::FuseStat(_) => Kind::FuseStat,
        }
    }
}
