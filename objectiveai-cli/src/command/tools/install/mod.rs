//! `tools install` sub-tier. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/tools/install/mod.rs`. Both
//! leaves (`filesystem`, `github`) are unary; this dispatcher returns a
//! one-shot stream wrapping the chosen leaf's `Response`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::tools::install::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod filesystem;
pub mod github;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Filesystem(req) => {
            let value = filesystem::execute(ctx, req).await?;
            once(Ok(Response::Filesystem(value)))
        }
        Request::FilesystemRequestSchema(req) => {
            let value = filesystem::request_schema::execute(ctx, req).await?;
            once(Ok(Response::FilesystemRequestSchema(value)))
        }
        Request::FilesystemResponseSchema(req) => {
            let value = filesystem::response_schema::execute(ctx, req).await?;
            once(Ok(Response::FilesystemResponseSchema(value)))
        }
        Request::Github(req) => {
            let value = github::execute(ctx, req).await?;
            once(Ok(Response::Github(value)))
        }
        Request::GithubRequestSchema(req) => {
            let value = github::request_schema::execute(ctx, req).await?;
            once(Ok(Response::GithubRequestSchema(value)))
        }
        Request::GithubResponseSchema(req) => {
            let value = github::response_schema::execute(ctx, req).await?;
            once(Ok(Response::GithubResponseSchema(value)))
        }
    };
    Ok(stream)
}
