//! `plugins daemon` sub-tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/plugins/daemon/mod.rs`. One leaf:
//! `notify` (unary).

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::plugins::daemon::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod notify;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Notify(req) => {
            let value = notify::execute(ctx, req).await?;
            once(Ok(Response::Notify(value)))
        }
        Request::NotifyRequestSchema(req) => {
            let value = notify::request_schema::execute(ctx, req).await?;
            once(Ok(Response::NotifyRequestSchema(value)))
        }
        Request::NotifyResponseSchema(req) => {
            let value = notify::response_schema::execute(ctx, req).await?;
            once(Ok(Response::NotifyResponseSchema(value)))
        }
    };
    Ok(stream)
}
