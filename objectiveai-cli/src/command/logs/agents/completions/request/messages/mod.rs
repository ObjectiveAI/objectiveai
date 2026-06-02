//! `logs agents completions request messages` sub-tier — media multiplexer.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::logs::agents::completions::request::messages::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod audio;
pub mod file;
pub mod image;
pub mod text;
pub mod video;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Audio(req) => {
            let inner = audio::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Audio)))
        }
        Request::File(req) => {
            let inner = file::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::File)))
        }
        Request::Image(req) => {
            let inner = image::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Image)))
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
