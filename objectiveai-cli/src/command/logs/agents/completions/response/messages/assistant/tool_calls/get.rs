//! `logs agents completions response messages assistant tool_calls get` — read a stored log record from disk.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .read_agent_completion_message_assistant_tool_call(&request.id, request.message_index, request.tool_call_index)
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::assistant::tool_calls::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
