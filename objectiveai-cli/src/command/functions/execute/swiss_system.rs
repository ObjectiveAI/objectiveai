//! `functions execute swiss_system` — bare-naked
//! chunk-or-id streaming handler. Same shape as `standard`, with
//! `Strategy::SwissSystem { pool, rounds }` instead of the default.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::cli::command::functions::execute::swiss_system::{
    Request, RequestInput, ResponseItem,
};
use objectiveai_sdk::functions::executions::request::{
    FunctionExecutionCreateParams, Strategy,
};

use crate::context::Context;
use crate::error::Error;
use crate::streaming::{InstanceItem, instance_subprocess_stream};

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let (function, profile) = tokio::try_join!(
        super::resolve_function(ctx, request.function),
        super::resolve_profile(ctx, request.profile),
    )?;
    let input = match request.input {
        RequestInput::Inline(v) => v,
        RequestInput::File(path) => super::resolve_input_file(path)?,
        RequestInput::PythonInline(code) => super::resolve_input_python_inline(code)?,
        RequestInput::PythonFile(path) => super::resolve_input_python_file(path)?,
    };

    let params = FunctionExecutionCreateParams {
        function,
        profile,
        retry_token: request.retry_token,
        from_cache: None,
        reasoning: None,
        strategy: Some(Strategy::SwissSystem {
            pool: request.pool,
            rounds: request.rounds,
        }),
        input,
        split: if request.split { Some(true) } else { None },
        invert: if request.invert { Some(true) } else { None },
        provider: None,
        seed: request.seed,
        stream: Some(true),
        continuation: request.continuation,
    };

    let stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);

    let raw = instance_subprocess_stream(
        ctx,
        crate::instance::request::InstanceEndpoint::FunctionsExecute(params),
        None,
        stream,
    );
    Ok(Box::pin(raw.map(map_item)))
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineJson(e)),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::execute::swiss_system as sdk;
    use objectiveai_sdk::cli::command::functions::execute::swiss_system::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::execute::swiss_system as sdk;
    use objectiveai_sdk::cli::command::functions::execute::swiss_system::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
