//! `logs agents completions response messages clear` — clear a category of stored log records.

use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::clear::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    ctx.filesystem.clear_agent_completion_messages().await?;
    Ok(Response {})
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::clear as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::clear::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::clear as sdk;
    use objectiveai_sdk::cli::command::logs::agents::completions::response::messages::clear::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
