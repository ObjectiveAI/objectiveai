//! `plugins get` — read one installed plugin's manifest by name.
//! Returns `None` if not installed.

use objectiveai_sdk::cli::command::plugins::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let manifest = match ctx.filesystem.get_plugin(&request.name).await {
        Some(m) => m,
        None => return Ok(None),
    };
    // CLI's `ManifestWithNameAndSource` and SDK's `ResponseManifest`
    // share the same on-disk shape — round-trip through JSON does the
    // conversion without hand-coding each field.
    let value = serde_json::to_value(&manifest)
        .map_err(|e| Error::InlineJson(e))?;
    Ok(Some(
        serde_json::from_value(value).map_err(|e| Error::InlineJson(e))?,
    ))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::plugins::get as sdk;
    use objectiveai_sdk::cli::command::plugins::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::plugins::get as sdk;
    use objectiveai_sdk::cli::command::plugins::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
