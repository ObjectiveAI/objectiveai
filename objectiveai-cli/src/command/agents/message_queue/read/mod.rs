//! `agents message-queue read` sub-tier CLI dispatch. One leaf today: `id`.

use std::pin::Pin;

use futures::{Stream, stream};
use objectiveai_sdk::cli::command::agents::message_queue::read::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod id;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Id(req) => {
            let value = id::execute(ctx, req).await?;
            once(Ok(ResponseItem::Id(value)))
        }
        Request::IdRequestSchema(req) => {
            let value = id::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::IdRequestSchema(value)))
        }
        Request::IdResponseSchema(req) => {
            let value = id::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::IdResponseSchema(value)))
        }
    };
    Ok(stream)
}
