//! `logs agents completions request subscribe` — wait (up to
//! `timeout_ms`) for the request log file to appear or be modified,
//! then read it as an `AgentCompletionCreateParamsLog`. Timeout
//! becomes [`Error::Filesystem(LogSubscribeTimedOut)`].

use std::time::Duration;

use objectiveai_sdk::cli::command::logs::agents::completions::request::subscribe::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .subscribe_agent_completion_request(
            &request.id,
            Duration::from_millis(request.timeout_ms),
            request.require_modification,
        )
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::request::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::request::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
