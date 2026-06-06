//! `logs agents completions response messages` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod assistant;
pub mod tool;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Assistant(req) => {
            let inner = assistant::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Assistant)))
        }
        Request::Tool(req) => {
            let inner = tool::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Tool)))
        }
    };
    Ok(stream)
}
