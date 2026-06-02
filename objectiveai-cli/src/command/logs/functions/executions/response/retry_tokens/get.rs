//! `logs functions executions response retry_tokens get` — bare-naked handler stub.

use objectiveai_sdk::cli::command::logs::functions::executions::response::retry_tokens::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("logs functions executions response retry_tokens get execute")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::response::retry_tokens::get as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::response::retry_tokens::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::logs::functions::executions::response::retry_tokens::get as sdk;
    use objectiveai_sdk::cli::command::logs::functions::executions::response::retry_tokens::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
