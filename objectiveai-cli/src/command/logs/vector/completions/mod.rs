//! `logs vector completions` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::vector::completions::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod request;
pub mod response;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Request(req) => {
            let inner = self::request::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Request)))
        }
        Request::Response(req) => {
            let inner = response::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Response)))
        }
    };
    Ok(stream)
}
