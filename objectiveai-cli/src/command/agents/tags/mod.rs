//! `agents tags` — CLI-side dispatch for the tags subtree.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::tags::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod add;
pub mod lookup;

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
    };
    Ok(stream)
}
