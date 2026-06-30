//! `agents mcp servers` — CLI-side dispatch for listing the agent's
//! connected MCP servers. One leaf, `list`, does a socket round-trip to the
//! agent's per-`response_id` MCP listener and returns the proxy-local
//! `ListServersResult`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::mcp::servers::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;

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
    };
    Ok(stream)
}
