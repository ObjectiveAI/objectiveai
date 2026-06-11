//! `config api claude-agent-sdk` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::config::api::claude_agent_sdk::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub mod get;
pub mod set;

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
        Request::Set(req) => {
            let value = set::execute(ctx, req).await?;
            once(Ok(Response::Set(value)))
        }
        Request::SetRequestSchema(req) => {
            let value = set::request_schema::execute(ctx, req).await?;
            once(Ok(Response::SetRequestSchema(value)))
        }
        Request::SetResponseSchema(req) => {
            let value = set::response_schema::execute(ctx, req).await?;
            once(Ok(Response::SetResponseSchema(value)))
        }
    };
    Ok(stream)
}
