//! `laboratories` — top-level CLI dispatch for laboratory containers (podman
//! containers the conduit dials as client-side MCP servers). Distinct from
//! `agents laboratories` (attachments). Only `create` exists today.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::laboratories::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod create;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Create(req) => {
            let value = create::execute(ctx, req).await?;
            once(Ok(ResponseItem::Create(value)))
        }
        Request::CreateRequestSchema(req) => {
            let value = create::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::CreateRequestSchema(value)))
        }
        Request::CreateResponseSchema(req) => {
            let value = create::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::CreateResponseSchema(value)))
        }
    };
    Ok(stream)
}
