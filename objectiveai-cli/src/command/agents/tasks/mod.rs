//! `agents tasks` — CLI-side dispatch for the tasks subtree.
//! Two leaves today: `schedule` and `list`.

use std::pin::Pin;

use futures::{Stream, stream};
use objectiveai_sdk::cli::command::agents::tasks::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod list;
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
        Request::List(req) => {
            // `list::execute` returns the rows up-front; emit one
            // stream item per row.
            let items = list::execute(ctx, req).await?;
            Box::pin(stream::iter(
                items.into_iter().map(|r| Ok(ResponseItem::List(r))),
            ))
        }
        Request::ListRequestSchema(req) => {
            let value = list::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListRequestSchema(value)))
        }
        Request::ListResponseSchema(req) => {
            let value = list::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::ListResponseSchema(value)))
        }
    };
    Ok(stream)
}
