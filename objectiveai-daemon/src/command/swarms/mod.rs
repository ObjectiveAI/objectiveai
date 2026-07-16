//! `swarms` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/swarms/mod.rs`. `list` is a
//! streaming leaf (one `ResponseItem` per swarm); `get` and `publish`
//! are unary.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::swarms::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod get;
pub mod list;
pub mod publish;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
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
        Request::Publish(req) => {
            let value = publish::execute(ctx, req).await?;
            once(Ok(ResponseItem::Publish(value)))
        }
        Request::PublishRequestSchema(req) => {
            let value = publish::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PublishRequestSchema(value)))
        }
        Request::PublishResponseSchema(req) => {
            let value = publish::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::PublishResponseSchema(value)))
        }
    };
    Ok(stream)
}
