//! `logs agents completions response messages subscribe` — wait (up to `timeout_ms`) for a log file to appear
//! or be modified, then read it. Timeout becomes
//! [`Error::Filesystem(LogSubscribeTimedOut)`].

use std::time::Duration;

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::subscribe::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .subscribe_agent_completion_message(
            &request.id, request.message_index,
            Duration::from_millis(request.timeout_ms),
            request.require_modification,
        )
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::subscribe as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
