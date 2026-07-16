//! `config api mcp-call-timeout-ms set` — write `api.mcp_call_timeout_ms` to on-disk config.
//!
//! The value is a millisecond integer; we parse it to a `u64` here (so a
//! non-numeric value fails the `set` with a clear error rather than
//! landing on disk) before persisting it.

use objectiveai_sdk::cli::command::api::config::mcp_call_timeout_ms::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let timeout_ms: u64 = {
        let mut de = serde_json::Deserializer::from_str(&request.value);
        serde_path_to_error::deserialize(&mut de).map_err(Error::InlineDeserialize)?
    };
    let mut config = ctx.filesystem.read_config_at(request.scope).await?;
    config.api().set_mcp_call_timeout_ms(timeout_ms);
    ctx.filesystem.write_config_at(request.scope, &config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_call_timeout_ms::set as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_call_timeout_ms::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::api::config::mcp_call_timeout_ms::set as sdk;
    use objectiveai_sdk::cli::command::api::config::mcp_call_timeout_ms::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
