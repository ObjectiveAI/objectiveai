//! `functions execute` sub-tier. Standard and SwissSystem are
//! chunk-or-id streaming leaves; their bare-naked `execute` returns
//! `Stream<ResponseItem>` and the inner stream is mapped into the
//! tier's `ResponseItem` directly.

use std::path::PathBuf;
use std::pin::Pin;

use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::functions::execute::{
    FunctionSpec, ProfileSpec, Request, ResponseItem,
};
use objectiveai_sdk::functions::expression::InputValue;
use objectiveai_sdk::functions::{
    FullInlineFunctionOrRemoteCommitOptional, InlineProfileOrRemoteCommitOptional,
};

use crate::context::Context;
use crate::error::Error;

pub(super) mod runner;
pub mod standard;
pub mod swiss_system;

pub(super) async fn resolve_function(
    ctx: &Context,
    spec: FunctionSpec,
) -> Result<FullInlineFunctionOrRemoteCommitOptional, Error> {
    match spec {
        FunctionSpec::Resolved(r) => Ok(r),
        FunctionSpec::File(path) => read_json_file(&path),
        FunctionSpec::PythonInline(code) => ctx.python().await?.exec_code(ctx, &code, None::<()>).await?.ok_or(Error::PythonNoOutput),
        FunctionSpec::PythonFile(path) => ctx.python().await?.exec_file(ctx, &path, None::<()>).await?.ok_or(Error::PythonNoOutput),
    }
}

pub(super) async fn resolve_profile(
    ctx: &Context,
    spec: ProfileSpec,
) -> Result<InlineProfileOrRemoteCommitOptional, Error> {
    match spec {
        ProfileSpec::Resolved(r) => Ok(r),
        ProfileSpec::File(path) => read_json_file(&path),
        ProfileSpec::PythonInline(code) => ctx.python().await?.exec_code(ctx, &code, None::<()>).await?.ok_or(Error::PythonNoOutput),
        ProfileSpec::PythonFile(path) => ctx.python().await?.exec_file(ctx, &path, None::<()>).await?.ok_or(Error::PythonNoOutput),
    }
}

pub(super) fn resolve_input_file(path: PathBuf) -> Result<InputValue, Error> {
    read_json_file(&path)
}

pub(super) async fn resolve_input_python_inline(
    ctx: &Context,
    code: String,
) -> Result<InputValue, Error> {
    ctx.python().await?.exec_code(ctx, &code, None::<()>).await?.ok_or(Error::PythonNoOutput)
}

pub(super) async fn resolve_input_python_file(
    ctx: &Context,
    path: PathBuf,
) -> Result<InputValue, Error> {
    ctx.python().await?.exec_file(ctx, &path, None::<()>).await?.ok_or(Error::PythonNoOutput)
}

/// Read a JSON file and deserialize as `T`. Used by every `*-file`
/// resolution arm above. `serde_path_to_error` preserves the
/// in-document location of the failure so error messages point at
/// the offending field.
fn read_json_file<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Result<T, Error> {
    let bytes = std::fs::read(path)
        .map_err(|e| Error::JsonFileRead(path.to_path_buf(), e))?;
    let mut de = serde_json::Deserializer::from_slice(&bytes);
    serde_path_to_error::deserialize(&mut de).map_err(Error::InlineDeserialize)
}

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

fn once<T: Send + 'static>(
    item: Result<T, Error>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>> {
    Box::pin(futures::stream::once(async move { item }))
}

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let stream: ItemStream = match request {
        Request::Standard(req) => {
            let inner = standard::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::Standard)))
        }
        Request::StandardRequestSchema(req) => {
            let value = standard::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::StandardRequestSchema(value)))
        }
        Request::StandardResponseSchema(req) => {
            let value = standard::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::StandardResponseSchema(value)))
        }
        Request::SwissSystem(req) => {
            let inner = swiss_system::execute(ctx, req).await?;
            Box::pin(inner.map(|r| r.map(ResponseItem::SwissSystem)))
        }
        Request::SwissSystemRequestSchema(req) => {
            let value = swiss_system::request_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SwissSystemRequestSchema(value)))
        }
        Request::SwissSystemResponseSchema(req) => {
            let value = swiss_system::response_schema::execute(ctx, req).await?;
            once(Ok(ResponseItem::SwissSystemResponseSchema(value)))
        }
    };
    Ok(stream)
}
