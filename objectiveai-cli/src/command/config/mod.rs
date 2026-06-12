//! `config` tier dispatch.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::config::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod api;
pub mod db;
pub mod mcp;
pub mod viewer;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Api(req) => {
            let inner = api::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Api)))
        }
        Request::Db(req) => {
            let inner = db::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Db)))
        }
        Request::Mcp(req) => {
            let inner = mcp::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Mcp)))
        }
        Request::Viewer(req) => {
            let inner = viewer::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Viewer)))
        }
    };
    Ok(stream)
}
