//! `functions inventions recursive` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::inventions::recursive::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod create;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Create(req) => {
            let inner = create::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Create)))
        }
    };
    Ok(stream)
}
