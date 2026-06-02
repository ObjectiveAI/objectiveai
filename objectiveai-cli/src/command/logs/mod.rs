//! `logs` tier dispatch.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod agents;
pub mod clear;
pub mod functions;
pub mod vector;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Agents(req) => {
            let inner = agents::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Agents)))
        }
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
        Request::Functions(req) => {
            let inner = functions::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Functions)))
        }
        Request::Vector(req) => {
            let inner = vector::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Vector)))
        }
    };
    Ok(stream)
}
