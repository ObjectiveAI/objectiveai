//! `agents tags` — CLI-side dispatch for the tags subtree.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::tags::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod apply;
pub mod lookup;
pub mod remove;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Lookup(req) => {
            let value = lookup::execute(ctx, req).await?;
            once(Ok(ResponseItem::Lookup(value)))
        }
        Request::LookupRequestSchema(req) => {
            let value = lookup::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::LookupRequestSchema(value)))
        }
        Request::LookupResponseSchema(req) => {
            let value = lookup::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::LookupResponseSchema(value)))
        }
        Request::Apply(req) => {
            let value = apply::execute(ctx, req).await?;
            once(Ok(ResponseItem::Apply(value)))
        }
        Request::ApplyRequestSchema(req) => {
            let value = apply::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ApplyRequestSchema(value)))
        }
        Request::ApplyResponseSchema(req) => {
            let value = apply::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ApplyResponseSchema(value)))
        }
        Request::Remove(req) => {
            let value = remove::execute(ctx, req).await?;
            once(Ok(ResponseItem::Remove(value)))
        }
        Request::RemoveRequestSchema(req) => {
            let value = remove::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoveRequestSchema(value)))
        }
        Request::RemoveResponseSchema(req) => {
            let value = remove::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoveResponseSchema(value)))
        }
    };
    Ok(stream)
}
