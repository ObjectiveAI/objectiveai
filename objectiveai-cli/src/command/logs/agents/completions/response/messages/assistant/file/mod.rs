//! `logs agents completions response messages assistant file` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::file::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod clear;
pub mod get;
pub mod subscribe;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Clear(req) => {
            let value = clear::execute(ctx, req).await?;
            once(Ok(Response::Clear(value)))
        }
        Request::ClearRequestSchema(req) => {
            let value = clear::request_schema::execute(ctx, req).await?;
            once(Ok(Response::ClearRequestSchema(value)))
        }
        Request::ClearResponseSchema(req) => {
            let value = clear::response_schema::execute(ctx, req).await?;
            once(Ok(Response::ClearResponseSchema(value)))
        }
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
        Request::Subscribe(req) => {
            let value = subscribe::execute(ctx, req).await?;
            once(Ok(Response::Subscribe(value)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(ctx, req).await?;
            once(Ok(Response::SubscribeRequestSchema(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(ctx, req).await?;
            once(Ok(Response::SubscribeResponseSchema(value)))
        }
    };
    Ok(stream)
}
