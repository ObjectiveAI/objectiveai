//! `daemon` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/daemon/mod.rs`. `spawn` is
//! streaming (the resident daemon yields readiness then serves);
//! `kill` is unary.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::daemon::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod kill;
pub mod spawn;

/// Per-state daemon singleton lock key (under `state_dir/locks`).
pub const DAEMON_LOCK_KEY: &str = "plugins-daemon";

/// Init gate key — serializes daemon startup so the singleton lock is
/// acquired without racing (mirrors `objectiveai-db`'s `db-init` gate).
pub const DAEMON_INIT_LOCK_KEY: &str = "plugins-daemon-init";

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Spawn(req) => {
            let inner = spawn::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Spawn)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SpawnResponseSchema(value)))
        }
        Request::Kill(req) => {
            let value = kill::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Kill(value)))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::KillRequestSchema(value)))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::KillResponseSchema(value)))
        }
    };
    Ok(stream)
}
