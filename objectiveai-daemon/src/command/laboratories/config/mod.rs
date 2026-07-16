//! `laboratories config` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::laboratories::config::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod addresses;
pub mod local;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Addresses(req) => {
            let inner = addresses::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Addresses)))
        }
        Request::Local(req) => {
            let inner = local::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Local)))
        }
    };
    Ok(stream)
}
