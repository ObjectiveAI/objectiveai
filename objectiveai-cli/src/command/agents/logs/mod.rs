//! `agents logs` — CLI-side dispatch for the logs subtree. One
//! sub-tier today: `read`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::logs::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod read;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Read(req) => {
            let inner = read::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Read)))
        }
    };
    Ok(stream)
}
