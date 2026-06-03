//! `logs clear` — clear every category of log records across all
//! endpoints (agents, vector, functions). Concurrent fan-out via
//! `try_join_all`; sums the per-category record counts into the SDK
//! `Response.count`.

use objectiveai_sdk::cli::command::logs::clear::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let fs = &ctx.filesystem;
    type ClearFuture<'a> =
        std::pin::Pin<Box<dyn std::future::Future<Output = Result<u64, crate::filesystem::Error>> + Send + 'a>>;
    let futures: Vec<ClearFuture<'_>> = vec![
        Box::pin(fs.clear_agent_completions()),
        Box::pin(fs.clear_agent_completion_continuations()),
        Box::pin(fs.clear_agent_completion_messages()),
        Box::pin(fs.clear_agent_completion_message_logprobs()),
        Box::pin(fs.clear_agent_completion_message_reasoning()),
        Box::pin(fs.clear_agent_completion_message_refusal()),
        Box::pin(fs.clear_agent_completion_message_tool_calls()),
        Box::pin(fs.clear_agent_completion_message_images()),
        Box::pin(fs.clear_agent_completion_message_audio()),
        Box::pin(fs.clear_agent_completion_message_video()),
        Box::pin(fs.clear_agent_completion_message_files()),
        Box::pin(fs.clear_vector_completions()),
        Box::pin(fs.clear_function_executions()),
        Box::pin(fs.clear_function_execution_retry_tokens()),
        Box::pin(fs.clear_function_inventions()),
        Box::pin(fs.clear_function_inventions_recursive()),
    ];
    let counts = futures::future::try_join_all(futures).await?;
    Ok(Response {
        count: counts.into_iter().sum(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::clear as sdk;
    use objectiveai_sdk::cli::command::logs::clear::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::clear as sdk;
    use objectiveai_sdk::cli::command::logs::clear::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
