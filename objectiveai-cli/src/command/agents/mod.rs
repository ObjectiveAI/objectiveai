//! `agents` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/agents/mod.rs`. Mix of unary
//! leaves (`get`, `me`, `message`, `publish`) and streaming sub-trees
//! (`list`, `read`, `spawn` ÃƒÂ¢Ã¢â€šÂ¬Ã¢â‚¬Â spawn is the chunk-or-id leaf whose
//! `execute` decides streaming vs unary internally based on
//! `dangerous_advanced.stream`).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod get;
pub mod list;
pub mod me;
pub mod message;
pub mod publish;
pub mod message_queue;
pub mod read;
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
        Request::List(req) => {
            let inner = list::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
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
        Request::Message(req) => {
            let inner = message::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Message)))
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
        Request::MessageQueue(req) => {
            let inner = message_queue::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::MessageQueue)))
        }
        Request::Read(req) => {
            let inner = read::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Read)))
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
