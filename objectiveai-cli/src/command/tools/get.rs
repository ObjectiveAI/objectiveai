//! `tools get` — read one installed tool's manifest by name.
//! Returns `None` if not installed.

use objectiveai_sdk::cli::command::tools::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let manifest = match ctx.filesystem.get_tool(&request.name).await {
        Some(m) => m,
        None => return Ok(None),
    };
    // Same on-disk shape on both sides of the boundary — JSON
    // round-trip handles the field-by-field conversion.
    let value = serde_json::to_value(&manifest)
        .map_err(|e| Error::InlineJson(e))?;
    Ok(Some(
        serde_json::from_value(value).map_err(|e| Error::InlineJson(e))?,
    ))
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
