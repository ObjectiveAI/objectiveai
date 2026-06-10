//! `plugins get` — read one installed plugin's manifest by
//! `(owner, name, version)`. Returns `None` if not installed.

use objectiveai_sdk::cli::command::plugins::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx
        .filesystem
        .get_plugin(&request.owner, &request.name, &request.version)
        .await
        .map(Into::into))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::get as sdk;
    use objectiveai_sdk::cli::command::plugins::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::get as sdk;
    use objectiveai_sdk::cli::command::plugins::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
