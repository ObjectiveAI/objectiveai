//! `config api mcp-backoff set` — write `api.mcp_backoff` to on-disk config.
//!
//! Unlike the string-valued config leaves (e.g. `address`), the value is
//! a single JSON blob; we parse it into an `mcp::Backoff` here (so an
//! invalid blob fails the `set` with a clear path-annotated error rather
//! than landing an unusable value on disk) before persisting it.

use objectiveai_sdk::cli::command::api::config::mcp_backoff::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let backoff: objectiveai_sdk::mcp::Backoff = {
        let mut de = serde_json::Deserializer::from_str(&request.value);
        serde_path_to_error::deserialize(&mut de).map_err(Error::InlineDeserialize)?
    };
    let mut config = ctx.filesystem.read_config_at(request.scope).await?;
    config.api().set_mcp_backoff(backoff);
    ctx.filesystem.write_config_at(request.scope, &config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::set as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::set as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_backoff::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
