//! `daemon config` sub-tier.

use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::daemon::config::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod address;
pub mod get;
pub mod refresh_secret_signature_pair;
pub mod secret;
pub mod signature;

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
        Request::Address(req) => {
            let inner = address::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Address)))
        }
        Request::RefreshSecretSignaturePair(req) => {
            let value = refresh_secret_signature_pair::execute(global, scoped, req).await?;
            once(Ok(Response::RefreshSecretSignaturePair(value)))
        }
        Request::RefreshSecretSignaturePairRequestSchema(req) => {
            let value =
                refresh_secret_signature_pair::request_schema::execute(global, scoped, req)
                    .await?;
            once(Ok(Response::RefreshSecretSignaturePairRequestSchema(value)))
        }
        Request::RefreshSecretSignaturePairResponseSchema(req) => {
            let value =
                refresh_secret_signature_pair::response_schema::execute(global, scoped, req)
                    .await?;
            once(Ok(Response::RefreshSecretSignaturePairResponseSchema(value)))
        }
        Request::Secret(req) => {
            let inner = secret::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Secret)))
        }
        Request::Signature(req) => {
            let inner = signature::execute(global, scoped, req).await?;
            Box::pin(inner.map(|r| r.map(Response::Signature)))
        }
    };
    Ok(stream)
}
