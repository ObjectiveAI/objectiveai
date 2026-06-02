//! `config agents` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::config::agents::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod favorites;
pub mod get;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Get(req) => {
            let inner = get::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Get)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
        }
        Request::Favorites(req) => {
            let inner = favorites::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Favorites)))
        }
    };
    Ok(stream)
}
