//! `functions` tier dispatch.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod execute;
pub mod get;
pub mod list;
pub mod profiles;
pub mod publish;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Execute(req) => {
            let inner = execute::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Execute)))
        }
        Request::Get(req) => {
            let value = get::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Get(value)))
        }
        Request::GetRequestSchema(req) => {
            let value = get::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::GetRequestSchema(value)))
        }
        Request::GetResponseSchema(req) => {
            let value = get::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::GetResponseSchema(value)))
        }
        Request::List(req) => {
            let inner = list::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::List)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Profiles(req) => {
            let inner = profiles::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Profiles)))
        }
        Request::Publish(req) => {
            let value = publish::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Publish(value)))
        }
        Request::PublishRequestSchema(req) => {
            let value = publish::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PublishRequestSchema(value)))
        }
        Request::PublishResponseSchema(req) => {
            let value = publish::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::PublishResponseSchema(value)))
        }
    };
    Ok(stream)
}
