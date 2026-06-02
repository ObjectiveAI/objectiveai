//! `logs agents completions response messages logprobs clear` — clear a category of stored log records; returns the
//! number of records removed.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::logprobs::clear::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let count = ctx.filesystem.clear_agent_completion_message_logprobs().await?;
    Ok(Response { count })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::logprobs::clear as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::logprobs::clear::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::logprobs::clear as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::logprobs::clear::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
