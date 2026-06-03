//! `agents list` sub-tier. Both leaves (`active`, `available`) are
//! streaming.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod active;
pub mod available;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Active(req) => {
            let inner = active::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Active)))
        }
        Request::ActiveRequestSchema(req) => {
            let value = active::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ActiveRequestSchema(value)))
        }
        Request::ActiveResponseSchema(req) => {
            let value = active::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ActiveResponseSchema(value)))
        }
        Request::Available(req) => {
            let inner = available::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Available)))
        }
        Request::AvailableRequestSchema(req) => {
            let value = available::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AvailableRequestSchema(value)))
        }
        Request::AvailableResponseSchema(req) => {
            let value = available::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AvailableResponseSchema(value)))
        }
    };
    Ok(stream)
}
