//! `logs functions` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::functions::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod executions;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Executions(req) => {
            let inner = executions::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Executions)))
        }
    };
    Ok(stream)
}
