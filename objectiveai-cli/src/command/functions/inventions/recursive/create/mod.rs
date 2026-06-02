//! `functions inventions recursive create` sub-tier. AlphaScalar,
//! AlphaVector, and Remote are chunk-or-id streaming leaves.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::inventions::recursive::create::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

pub mod alpha_scalar;
pub mod alpha_vector;
pub mod remote;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::AlphaScalar(req) => {
            let inner = alpha_scalar::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::AlphaScalar)))
        }
        Request::AlphaScalarRequestSchema(req) => {
            let value = alpha_scalar::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaScalarRequestSchema(value)))
        }
        Request::AlphaScalarResponseSchema(req) => {
            let value = alpha_scalar::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaScalarResponseSchema(value)))
        }
        Request::AlphaVector(req) => {
            let inner = alpha_vector::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::AlphaVector)))
        }
        Request::AlphaVectorRequestSchema(req) => {
            let value = alpha_vector::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaVectorRequestSchema(value)))
        }
        Request::AlphaVectorResponseSchema(req) => {
            let value = alpha_vector::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::AlphaVectorResponseSchema(value)))
        }
        Request::Remote(req) => {
            let inner = remote::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Remote)))
        }
        Request::RemoteRequestSchema(req) => {
            let value = remote::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoteRequestSchema(value)))
        }
        Request::RemoteResponseSchema(req) => {
            let value = remote::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::RemoteResponseSchema(value)))
        }
    };
    Ok(stream)
}
