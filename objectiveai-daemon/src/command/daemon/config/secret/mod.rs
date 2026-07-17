//! `daemon config secret` sub-tier.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::daemon::config::secret::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod get;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
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
