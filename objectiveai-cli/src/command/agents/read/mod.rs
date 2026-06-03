//! `agents read` sub-tier. `all`, `pending`, `subscribe` are streaming;
//! `id` is unary.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::read::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod all;
pub mod id;
pub mod pending;
pub mod subscribe;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::All(req) => {
            let inner = all::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::All)))
        }
        Request::AllRequestSchema(req) => {
            let value = all::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AllRequestSchema(value)))
        }
        Request::AllResponseSchema(req) => {
            let value = all::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AllResponseSchema(value)))
        }
        Request::Id(req) => {
            let value = id::execute(ctx, req).await?;
            once(Ok(ResponseItem::Id(value)))
        }
        Request::IdRequestSchema(req) => {
            let value = id::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::IdRequestSchema(value)))
        }
        Request::IdResponseSchema(req) => {
            let value = id::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::IdResponseSchema(value)))
        }
        Request::Pending(req) => {
            let inner = pending::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Pending)))
        }
        Request::PendingRequestSchema(req) => {
            let value = pending::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PendingRequestSchema(value)))
        }
        Request::PendingResponseSchema(req) => {
            let value = pending::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PendingResponseSchema(value)))
        }
        Request::Subscribe(req) => {
            let inner = subscribe::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SubscribeRequestSchema(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SubscribeResponseSchema(value)))
        }
    };
    Ok(stream)
}
