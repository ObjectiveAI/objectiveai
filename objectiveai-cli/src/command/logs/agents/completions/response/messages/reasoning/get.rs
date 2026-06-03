//! `logs agents completions response messages reasoning get` — read a stored log record from disk.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::reasoning::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .read_agent_completion_message_reasoning(&request.id, request.message_index)
        .await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::reasoning::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::reasoning::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::reasoning::get as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::reasoning::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
