//! `api` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/api/mod.rs` — matches on the
//! SDK's tier `Request` variants and dispatches to local leaf
//! `execute`s. All api leaves are unary, so every arm wraps a single
//! value in a one-shot stream.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::api::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod kill;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Kill(req) => {
            let value = kill::execute(ctx, req).await?;
            once(Ok(Response::Kill(value)))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(ctx, req).await?;
            once(Ok(Response::KillRequestSchema(value)))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(ctx, req).await?;
            once(Ok(Response::KillResponseSchema(value)))
        }
        Request::Spawn(req) => {
            let value = spawn::execute(ctx, req).await?;
            once(Ok(Response::Spawn(value)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(ctx, req).await?;
            once(Ok(Response::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(ctx, req).await?;
            once(Ok(Response::SpawnResponseSchema(value)))
        }
    };
    Ok(stream)
}
