//! `logs vector completions response` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::vector::completions::response::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod clear;
pub mod get;
pub mod list;
pub mod subscribe;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Clear(req) => {
            let value = clear::execute(ctx, req).await?;
            once(Ok(ResponseItem::Clear(value)))
        }
        Request::ClearRequestSchema(req) => {
            let value = clear::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ClearRequestSchema(value)))
        }
        Request::ClearResponseSchema(req) => {
            let value = clear::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ClearResponseSchema(value)))
        }
        Request::Get(req) => {
            let value = get::execute(ctx, req).await?;
            once(Ok(ResponseItem::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
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
        Request::Subscribe(req) => {
            let value = subscribe::execute(ctx, req).await?;
            once(Ok(ResponseItem::Subscribe(value)))
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
