//! `functions execute swiss-system` — in-process chunk-or-id
//! streaming handler. Same two-mode shape as `standard`, with
//! `Strategy::SwissSystem { pool, rounds }` instead of the default.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::cli::command::functions::execute::swiss_system::{
    Request, RequestDangerousAdvanced, RequestInput, ResponseItem,
};
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};
use objectiveai_sdk::functions::executions::request::{
    FunctionExecutionCreateParams, Strategy,
};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let want_stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    if want_stream {
        execute_streaming(ctx, request).await
    } else {
        execute_detached(request).await
    }
}

async fn execute_streaming(
    ctx: &Context,
    request: Request,
) -> Result<ItemStream, Error> {
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
    let agents_dir = ctx
        .filesystem
        .base_dir()
        .join("instances")
        .join("agents");
    let inner = super::runner::run(ctx.clone(), params, agents_dir);
    Ok(Box::pin(inner.map(|r| {
        r.map(|ev| match ev {
            super::runner::Event::Id(id) => ResponseItem::Id(id),
            super::runner::Event::Chunk(c) => ResponseItem::Chunk(c),
        })
    })))
}

/// Stream-false: self-respawn as a detached subprocess running
/// the same `functions execute swiss-system ...` argv with
/// `stream=true`. Take the first `ResponseItem::Id` off the
/// child's stdout, yield it, return. Child outlives this call.
async fn execute_detached(request: Request) -> Result<ItemStream, Error> {
    let mut child_request = request;
    match child_request.dangerous_advanced.as_mut() {
        Some(adv) => adv.stream = Some(true),
        None => {
            child_request.dangerous_advanced =
                Some(RequestDangerousAdvanced { stream: Some(true) })
        }
    }
    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let executor = BinaryExecutor::from_path(exe).detach(true);
    let mut stream = executor
        .execute::<Request, ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!(
            "self-respawn for functions execute swiss-system: {e}"
        )))?;
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!(
            "self-respawn for functions execute swiss-system: {e}"
        )))?;
    Ok(Box::pin(
        objectiveai_sdk::cli::command::StreamOnce::new(Ok(first)),
    ))
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
