//! `logs functions executions response clear` — clear a category of stored log records.

use objectiveai_sdk::cli::command::logs::functions::executions::response::clear::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    ctx.filesystem.clear_function_executions().await?;
    Ok(Response {})
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::response::clear as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::response::clear::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::response::clear as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::response::clear::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
