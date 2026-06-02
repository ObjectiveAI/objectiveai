//! `config` tier dispatch.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::config::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod agents;
pub mod functions;
pub mod mcp;
pub mod swarms;
pub mod viewer;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Agents(req) => {
            let inner = agents::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
        }
        Request::Functions(req) => {
            let inner = functions::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
        }
        Request::Mcp(req) => {
            let inner = mcp::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Swarms(req) => {
            let inner = swarms::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Swarms)))
        }
        Request::Viewer(req) => {
            let inner = viewer::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
        }
    };
    Ok(stream)
}
