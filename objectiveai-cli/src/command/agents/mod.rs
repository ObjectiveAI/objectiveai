//! `agents` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/agents/mod.rs`. Mix of unary
//! leaves (`get`, `publish`) and streaming sub-trees (`list`, `spawn`,
//! `message`, `logs`, `queue`, `tags`, plus the surviving
//! `instances` aggregate that owns `me`/`list`).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod get;
pub mod instances;
pub mod list;
pub mod locks;
pub mod logs;
pub mod message;
pub mod publish;
pub mod queue;
pub mod spawn;
pub mod tags;

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
        Request::Instances(req) => {
            let inner = instances::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Instances)))
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
        Request::Logs(req) => {
            let inner = logs::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Logs)))
        }
        Request::Message(req) => {
            let value = message::execute(ctx, req).await?;
            once(Ok(ResponseItem::Message(value)))
        }
        Request::MessageRequestSchema(req) => {
            let value = message::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::MessageRequestSchema(value)))
        }
        Request::MessageResponseSchema(req) => {
            let value = message::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::MessageResponseSchema(value)))
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
        Request::Queue(req) => {
            let inner = queue::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Queue)))
        }
        Request::Spawn(req) => {
            let inner = spawn::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Spawn)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnResponseSchema(value)))
        }
        Request::Tags(req) => {
            let inner = tags::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Tags)))
        }
    };
    Ok(stream)
}
