//! `db` — CLI-side dispatch for database access + lifecycle:
//! `query`, `spawn`, `kill`.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::db::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod kill;
pub mod query;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Kill(req) => {
            let value = kill::execute(ctx, req).await?;
            once(Ok(ResponseItem::Kill(value)))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillRequestSchema(value)))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::KillResponseSchema(value)))
        }
        Request::Query(req) => {
            let value = query::execute(ctx, req).await?;
            once(Ok(ResponseItem::Query(value)))
        }
        Request::QueryRequestSchema(req) => {
            let value = query::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::QueryRequestSchema(value)))
        }
        Request::QueryResponseSchema(req) => {
            let value = query::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::QueryResponseSchema(value)))
        }
        Request::Spawn(req) => {
            let value = spawn::execute(ctx, req).await?;
            once(Ok(ResponseItem::Spawn(value)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SpawnResponseSchema(value)))
        }
    };
    Ok(stream)
}
