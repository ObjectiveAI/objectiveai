//! `functions executions create standard` — bare-naked chunk-or-id
//! streaming handler.
//!
//! Same shape as `agents spawn`: spawns the instance runner as a
//! detached background process (unless `dangerous_advanced.stream =
//! Some(true)`, in which case the leaf follows the stream to EOF) and
//! yields one `ResponseItem::Id` then optionally per-chunk
//! `ResponseItem::Chunk` items.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use objectiveai_sdk::cli::command::functions::executions::create::standard::{
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
    let ctx = ctx.clone();
    let stream = async_stream::stream! {
        let resolved = tokio::try_join!(
            super::resolve_function(&ctx, request.function),
            super::resolve_profile(&ctx, request.profile),
        );
        let (function, profile) = match resolved {
            Ok(v) => v,
            Err(e) => { yield Err(e); return; }
        };
        let input = match request.input {
            RequestInput::Inline(v) => v,
            RequestInput::PythonInline(code) => {
                match super::resolve_input_python_inline(code) {
                    Ok(v) => v,
                    Err(e) => { yield Err(e); return; }
                }
            }
            RequestInput::PythonFile(path) => {
                match super::resolve_input_python_file(path) {
                    Ok(v) => v,
                    Err(e) => { yield Err(e); return; }
                }
            }
        };

        let params = FunctionExecutionCreateParams {
            function,
            profile,
            retry_token: request.retry_token,
            from_cache: None,
            reasoning: None,
            strategy: Some(Strategy::Default),
            input,
            split: if request.split { Some(true) } else { None },
            invert: if request.invert { Some(true) } else { None },
            provider: None,
            seed: request.seed,
            stream: Some(true),
            continuation: request.continuation,
        };

        let follow = request
            .dangerous_advanced
            .as_ref()
            .and_then(|a| a.stream)
            .unwrap_or(false);

        let mut raw = instance_subprocess_stream(
            &ctx,
            &["functions", "executions", "create", "standard"],
            &params,
            None,
            follow,
        );
        while let Some(item) = raw.next().await {
            yield map_item(item);
        }
    };
    Ok(Box::pin(stream))
}

fn map_item(item: Result<InstanceItem, Error>) -> Result<ResponseItem, Error> {
    match item? {
        InstanceItem::Id(id) => Ok(ResponseItem::Id(id)),
        InstanceItem::Chunk(value) => serde_json::from_value(value)
            .map(ResponseItem::Chunk)
            .map_err(|e| Error::InlineDeserialize(e.into())),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::executions::create::standard as sdk;
    use objectiveai_sdk::cli::command::functions::executions::create::standard::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::executions::create::standard as sdk;
    use objectiveai_sdk::cli::command::functions::executions::create::standard::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
