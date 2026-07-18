//! `channels logs` — daemon-side dispatch for the per-channel message
//! log. `request`/`reply` (writes), `list`/`open` (reads), `subscribe`
//! (long-poll).

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::channels::logs::{Request, ResponseItem};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod list;
pub mod open;
pub mod reply;
pub mod request;
pub mod subscribe;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(
    global: &GlobalContext,
    scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Request(req) => {
            let value = request::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Request(value)))
        }
        Request::RequestRequestSchema(req) => {
            let value = request::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::RequestRequestSchema(value)))
        }
        Request::RequestResponseSchema(req) => {
            let value = request::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::RequestResponseSchema(value)))
        }
        Request::Reply(req) => {
            let value = reply::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Reply(value)))
        }
        Request::ReplyRequestSchema(req) => {
            let value = reply::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ReplyRequestSchema(value)))
        }
        Request::ReplyResponseSchema(req) => {
            let value = reply::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ReplyResponseSchema(value)))
        }
        Request::List(req) => {
            let value = list::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::List(value)))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
        Request::Open(req) => {
            let value = open::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::Open(value)))
        }
        Request::OpenRequestSchema(req) => {
            let value = open::request_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::OpenRequestSchema(value)))
        }
        Request::OpenResponseSchema(req) => {
            let value = open::response_schema::execute(global, scoped, req).await?;
            once(Ok(ResponseItem::OpenResponseSchema(value)))
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
