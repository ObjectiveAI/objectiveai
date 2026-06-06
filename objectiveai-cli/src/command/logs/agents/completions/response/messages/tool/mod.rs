//! `logs agents completions response messages tool` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::tool::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod audio;
pub mod clear;
pub mod file;
pub mod get;
pub mod image;
pub mod subscribe;
pub mod text;
pub mod video;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Audio(req) => {
            let inner = audio::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Audio)))
        }
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
        Request::File(req) => {
            let inner = file::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::File)))
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
        Request::Image(req) => {
            let inner = image::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Image)))
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
        Request::Text(req) => {
            let inner = text::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Text)))
        }
        Request::Video(req) => {
            let inner = video::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Video)))
        }
    };
    Ok(stream)
}
