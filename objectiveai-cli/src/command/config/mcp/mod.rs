//! `config mcp` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::config::mcp::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod address;
pub mod get;
pub mod port;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Get(req) => {
            let value = get::execute(ctx, req).await?;
            once(Ok(Response::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(ctx, req).await?;
            once(Ok(Response::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(ctx, req).await?;
            once(Ok(Response::GetResponseSchema(value)))
        }
        Request::Address(req) => {
            let inner = address::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Address)))
        }
        Request::Port(req) => {
            let inner = port::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Port)))
        }
    };
    Ok(stream)
}
