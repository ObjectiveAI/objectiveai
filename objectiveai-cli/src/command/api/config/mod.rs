//! `config api` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::api::config::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod address;
pub mod commit_author_email;
pub mod commit_author_name;
pub mod get;
pub mod github_authorization;
pub mod http_referer;
pub mod mcp_authorization;
pub mod objectiveai_authorization;
pub mod openrouter_authorization;
pub mod user_agent;
pub mod x_title;

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
        Request::ObjectiveaiAuthorization(req) => {
            let inner = objectiveai_authorization::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::ObjectiveaiAuthorization)))
        }
        Request::OpenrouterAuthorization(req) => {
            let inner = openrouter_authorization::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::OpenrouterAuthorization)))
        }
        Request::GithubAuthorization(req) => {
            let inner = github_authorization::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::GithubAuthorization)))
        }
        Request::McpAuthorization(req) => {
            let inner = mcp_authorization::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::McpAuthorization)))
        }
        Request::UserAgent(req) => {
            let inner = user_agent::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::UserAgent)))
        }
        Request::HttpReferer(req) => {
            let inner = http_referer::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::HttpReferer)))
        }
        Request::XTitle(req) => {
            let inner = x_title::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::XTitle)))
        }
        Request::CommitAuthorName(req) => {
            let inner = commit_author_name::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::CommitAuthorName)))
        }
        Request::CommitAuthorEmail(req) => {
            let inner = commit_author_email::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(Response::CommitAuthorEmail)))
        }
    };
    Ok(stream)
}
