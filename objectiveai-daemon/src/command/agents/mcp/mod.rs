//! `agents mcp` — CLI-side dispatch for a live agent's aggregated MCP
//! surface. Sub-groups: `resources` (`list`, `read`), `servers` (`list`),
//! and `tools` (`call`, `list`). Each leaf does one socket round-trip to the
//! agent's per-`response_id` MCP listener and returns the MCP result.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::mcp::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod resources;
pub mod servers;
pub mod tools;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Resources(req) => {
            let inner = resources::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Resources)))
        }
        Request::Servers(req) => {
            let inner = servers::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Servers)))
        }
        Request::Tools(req) => {
            let inner = tools::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Tools)))
        }
    };
    Ok(stream)
}
