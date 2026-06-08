//! `agents instances` — CLI-side dispatch for the surviving
//! instances subtree. Two leaves: `me`, `list`. `spawn`, `message`,
//! and `read` moved up to `agents spawn`, `agents message`, and
//! `agents logs read`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::instances::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;
pub mod me;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Me(req) => {
            let value = me::execute(ctx, req).await?;
            once(Ok(ResponseItem::Me(value)))
        }
        Request::MeRequestSchema(req) => {
            let value = me::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::MeRequestSchema(value)))
        }
        Request::MeResponseSchema(req) => {
            let value = me::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::MeResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
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
