//! `functions profiles get` — read a profile definition by remote
//! path.

use objectiveai_sdk::cli::command::functions::profiles::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = request.path;
    Ok(objectiveai_sdk::functions::profiles::get_profile(&ctx.http, path).await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::functions::profiles::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
