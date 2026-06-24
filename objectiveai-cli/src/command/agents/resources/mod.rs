//! `agents resources` — CLI-side dispatch for the resource ops.
//! `list`, `read`. Each leaf does one socket round-trip to the agent's
//! per-`response_id` MCP listener and returns the MCP result.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::resources::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;
pub mod read;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::List(req) => {
            let value = list::execute(ctx, req).await?;
            once(Ok(ResponseItem::List(value)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Read(req) => {
            let value = read::execute(ctx, req).await?;
            once(Ok(ResponseItem::Read(value)))
        }
        Request::ReadRequestSchema(req) => {
            let value = read::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ReadRequestSchema(value)))
        }
        Request::ReadResponseSchema(req) => {
            let value = read::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ReadResponseSchema(value)))
        }
    };
    Ok(stream)
}
