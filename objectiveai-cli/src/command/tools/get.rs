//! `tools get` — read one installed tool's manifest by name.
//! Returns `None` if not installed.

use objectiveai_sdk::cli::command::tools::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    Ok(ctx.filesystem.get_tool(&request.name).await.map(Into::into))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tools::get as sdk;
    use objectiveai_sdk::cli::command::tools::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tools::get as sdk;
    use objectiveai_sdk::cli::command::tools::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
