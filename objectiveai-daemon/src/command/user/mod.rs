//! `user` tier — user requests.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::user::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod request;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Request(req) => {
            let value = request::execute(global, scoped, req).await?;
            once(Ok(Response::Request(value)))
        }
        Request::RequestRequestSchema(req) => {
            let value = request::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::RequestRequestSchema(value)))
        }
        Request::RequestResponseSchema(req) => {
            let value = request::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::RequestResponseSchema(value)))
        }
    };
    Ok(stream)
}
