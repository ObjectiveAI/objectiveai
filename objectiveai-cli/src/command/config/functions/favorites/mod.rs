//! `config functions favorites` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::config::functions::favorites::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod add;
pub mod del;
pub mod edit;
pub mod get;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Add(req) => {
            let value = add::execute(ctx, req).await?;
            once(Ok(ResponseItem::Add(value)))
        }
        Request::AddRequestSchema(req) => {
            let value = add::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AddRequestSchema(value)))
        }
        Request::AddResponseSchema(req) => {
            let value = add::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AddResponseSchema(value)))
        }
        Request::Del(req) => {
            let value = del::execute(ctx, req).await?;
            once(Ok(ResponseItem::Del(value)))
        }
        Request::DelRequestSchema(req) => {
            let value = del::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DelRequestSchema(value)))
        }
        Request::DelResponseSchema(req) => {
            let value = del::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::DelResponseSchema(value)))
        }
        Request::Edit(req) => {
            let value = edit::execute(ctx, req).await?;
            once(Ok(ResponseItem::Edit(value)))
        }
        Request::EditRequestSchema(req) => {
            let value = edit::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::EditRequestSchema(value)))
        }
        Request::EditResponseSchema(req) => {
            let value = edit::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::EditResponseSchema(value)))
        }
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
    };
    Ok(stream)
}
