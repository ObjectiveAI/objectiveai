//! `viewer` tier dispatch. Mirrors
//! `objectiveai-sdk-rs/src/cli/command/viewer/mod.rs`. All viewer leaves
//! are unary, so every arm wraps a single value in a one-shot stream.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::viewer::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub mod generate_secret_signature_pair;
pub mod kill;
pub mod spawn;

type ItemStream = Pin<Box<dyn Stream<Item = Result<Response, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::GenerateSecretSignaturePair(req) => {
            let value = generate_secret_signature_pair::execute(global, scoped, req).await?;
            once(Ok(Response::GenerateSecretSignaturePair(value)))
        }
        Request::GenerateSecretSignaturePairRequestSchema(req) => {
            let value = generate_secret_signature_pair::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::GenerateSecretSignaturePairRequestSchema(value)))
        }
        Request::GenerateSecretSignaturePairResponseSchema(req) => {
            let value = generate_secret_signature_pair::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::GenerateSecretSignaturePairResponseSchema(value)))
        }
        Request::Kill(req) => {
            let value = kill::execute(global, scoped, req).await?;
            once(Ok(Response::Kill(value)))
        }
        Request::KillRequestSchema(req) => {
            let value = kill::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::KillRequestSchema(value)))
        }
        Request::KillResponseSchema(req) => {
            let value = kill::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::KillResponseSchema(value)))
        }
        Request::Spawn(req) => {
            let value = spawn::execute(global, scoped, req).await?;
            once(Ok(Response::Spawn(value)))
        }
        Request::SpawnRequestSchema(req) => {
            let value = spawn::request_schema::execute(global, scoped, req).await?;
            once(Ok(Response::SpawnRequestSchema(value)))
        }
        Request::SpawnResponseSchema(req) => {
            let value = spawn::response_schema::execute(global, scoped, req).await?;
            once(Ok(Response::SpawnResponseSchema(value)))
        }
    };
    Ok(stream)
}
