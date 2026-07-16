//! `config api mcp-authorization` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::api::config::mcp_authorization::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod add;
pub mod del;
pub mod get;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Add(req) => {
            let value = add::execute(global, scoped, req).await?;
            once(Ok(Response::Add(value)))
        }
        Request::AddRequestSchema(req) => {
            let value = add::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::AddRequestSchema(value)))
        }
        Request::AddResponseSchema(req) => {
            let value = add::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::AddResponseSchema(value)))
        }
        Request::Del(req) => {
            let value = del::execute(global, scoped, req).await?;
            once(Ok(Response::Del(value)))
        }
        Request::DelRequestSchema(req) => {
            let value = del::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::DelRequestSchema(value)))
        }
        Request::DelResponseSchema(req) => {
            let value = del::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::DelResponseSchema(value)))
        }
        Request::Get(req) => {
            let value = get::execute(global, scoped, req).await?;
            once(Ok(Response::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::GetResponseSchema(value)))
        }
    };
    Ok(stream)
}
