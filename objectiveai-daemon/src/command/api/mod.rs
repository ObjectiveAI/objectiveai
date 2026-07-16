//! `api` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/api/mod.rs` — matches on the
//! SDK's tier `Request` variants and dispatches to local leaf
//! `execute`s. All api leaves are unary, so every arm wraps a single
//! value in a one-shot stream.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::api::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod config;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Config(req) => {
            let inner = config::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Config)))
        }
    };
    Ok(stream)
}
