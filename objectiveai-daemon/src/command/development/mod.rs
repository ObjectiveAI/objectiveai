//! `development` tier — daemon-side dispatch for development-mode
//! plugin registrations.
//!
//! `create`/`list`/`delete` touch only the in-process [`registry`];
//! `reset` is the one that leaves the daemon, forwarding to the LOCAL
//! laboratory host. All four require the resident daemon, since the
//! registry lives on its hubs and nowhere else.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::development::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

// The four leaf handlers live HERE rather than under `plugins/mcp/`
// so they sit beside the `registry` all of them touch; `plugins/mcp/`
// is dispatch only.
pub mod create;
pub mod delete;
pub mod list;
pub mod plugins;
pub mod registry;
pub mod reset;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    use futures::StreamExt;
    let stream: ItemStream = match request {
        Request::Plugins(req) => {
            let inner = plugins::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Plugins)))
        }
    };
    Ok(stream)
}
