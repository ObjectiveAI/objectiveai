//! `functions execute standard` — in-process chunk-or-id
//! streaming handler.
//!
//! Same two-mode shape as `agents spawn`: `stream=true` drives the
//! upstream WS directly via [`super::runner::run`] and yields each
//! chunk as it arrives; `stream=false` runs that same path as a
//! detached in-process daemon task
//! ([`crate::command::detached::spawn_detached`]), captures the first
//! `ResponseItem::Id`, yields it, and returns. The task outlives the
//! request and drives the execution to completion on the daemon's
//! runtime.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::cli::command::functions::execute::standard::{
    AgentInstanceHierarchy, AgentInstanceHierarchyType, Request,
    RequestDangerousAdvanced, RequestInput, ResponseItem,
};
use objectiveai_sdk::functions::executions::request::{
    FunctionExecutionCreateParams, Strategy,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let want_stream = request
        .dangerous_advanced
        .as_ref()
        .and_then(|a| a.stream)
        .unwrap_or(false);
    if want_stream {
        execute_streaming(global, scoped, request).await
    } else {
        execute_detached(global, scoped, request).await
    }
}

async fn execute_streaming(
    global: &GlobalContext, scoped: &ScopedContext,
    request: Request,
) -> Result<ItemStream, Error> {
    let (function, profile) = tokio::try_join!(
        super::resolve_function(global, scoped, request.function),
        super::resolve_profile(global, scoped, request.profile),
    )?;
    let input = match request.input {
        RequestInput::Inline(v) => v,
        RequestInput::File(path) => super::resolve_input_file(path)?,
        RequestInput::PythonInline(code) => super::resolve_input_python_inline(global, scoped, code).await?,
        RequestInput::PythonFile(path) => super::resolve_input_python_file(global, scoped, path).await?,
    };
    let seed = request.dangerous_advanced.as_ref().and_then(|a| a.seed);
    let params = FunctionExecutionCreateParams {
        function,
        profile,
        reasoning: None,
        strategy: Some(Strategy::Default),
        input,
        split: if request.split { Some(true) } else { None },
        invert: if request.invert { Some(true) } else { None },
        provider: None,
        seed,
        stream: Some(true),
        continuation: request.continuation,
    };
    let inner = super::runner::run(global.clone(), scoped.clone(), params);
    Ok(Box::pin(inner.map(|r| {
        r.map(|ev| match ev {
            super::runner::Event::Id(id) => ResponseItem::Id(id),
            super::runner::Event::Hierarchy(hier) => {
                ResponseItem::AgentInstanceHierarchy(AgentInstanceHierarchy {
                    r#type: AgentInstanceHierarchyType::AgentInstanceHierarchy,
                    agent_instance_hierarchy: hier,
                })
            }
            super::runner::Event::Chunk(c) => ResponseItem::Chunk(c),
        })
    })))
}

/// Stream-false: run the real streaming path (`stream=true`) as a
/// detached in-process daemon task
/// ([`crate::command::detached::spawn_detached`]), surface its first
/// item (the gated `Id`), and return. The task outlives this call and
/// drives the execution to completion on the daemon's runtime.
async fn execute_detached(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<ItemStream, Error> {
    let mut child_request = request;
    match child_request.dangerous_advanced.as_mut() {
        Some(adv) => adv.stream = Some(true),
        None => {
            child_request.dangerous_advanced = Some(RequestDangerousAdvanced {
                stream: Some(true),
                ..Default::default()
            })
        }
    }
    // The detached run re-enters via `crate::run` — strip the
    // parent-only envelope fields.
    crate::command::reexec::strip_inherited(&mut child_request.base);
    Ok(crate::command::detached::spawn_detached::<Request, ResponseItem>(
        global.clone(),
        scoped.clone(),
        child_request,
        |_| Some(true),
    ))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::execute::standard as sdk;
    use objectiveai_sdk::cli::command::functions::execute::standard::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::execute::standard as sdk;
    use objectiveai_sdk::cli::command::functions::execute::standard::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
