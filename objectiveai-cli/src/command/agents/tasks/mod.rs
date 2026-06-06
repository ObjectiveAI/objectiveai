//! `agents tasks` — CLI-side dispatch for the tasks subtree.
//! One leaf today: `schedule`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::tasks::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod schedule;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Schedule(req) => {
            let value = schedule::execute(ctx, req).await?;
            once(Ok(ResponseItem::Schedule(value)))
        }
        Request::ScheduleRequestSchema(req) => {
            let value = schedule::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ScheduleRequestSchema(value)))
        }
        Request::ScheduleResponseSchema(req) => {
            let value = schedule::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ScheduleResponseSchema(value)))
        }
    };
    Ok(stream)
}
