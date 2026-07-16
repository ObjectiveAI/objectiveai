//! `db` — CLI-side dispatch for database access + lifecycle:
//! `query`, `spawn`, `kill`.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::db::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod config;
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
        Request::Config(req) => {
            let inner = config::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Config)))
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
    };
    Ok(stream)
}
