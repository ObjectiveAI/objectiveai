//! `functions inventions` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::inventions::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod recursive;
pub mod state;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Recursive(req) => {
            let inner = recursive::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Recursive)))
        }
        Request::State(req) => {
            let inner = state::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::State)))
        }
    };
    Ok(stream)
}
