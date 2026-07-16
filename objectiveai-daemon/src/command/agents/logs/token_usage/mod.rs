//! `agents logs token-usage` — CLI-side dispatch. Leaf: `subscribe`
//! (wait for the stored `total_tokens` snapshot to change or the
//! instance lock to drop).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::agents::logs::token_usage::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod get;
pub mod subscribe;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
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
        Request::Subscribe(req) => {
            let inner = subscribe::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Subscribe)))
        }
        Request::SubscribeRequestSchema(req) => {
            let value = subscribe::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SubscribeRequestSchema(value)))
        }
        Request::SubscribeResponseSchema(req) => {
            let value = subscribe::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::SubscribeResponseSchema(value)))
        }
    };
    Ok(stream)
}
