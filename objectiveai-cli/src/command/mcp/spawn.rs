//! `mcp spawn` — start the `objectiveai-mcp` server in the background,
//! using `mcp.address` + `mcp.port` from on-disk config.

use objectiveai_sdk::cli::command::mcp::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;

    let address = config
        .mcp()
        .get_address()
        .ok_or(Error::MissingArgs(
            "mcp.address unset; run `objectiveai config mcp address set <addr>`",
        ))?
        .to_string();
    let port = config.mcp().get_port().ok_or(Error::MissingArgs(
        "mcp.port unset; run `objectiveai config mcp port set <port>`",
    ))?;

    crate::spawn::ensure_not_running("objectiveai-mcp")?;

    let bin = if cfg!(windows) {
        "objectiveai-mcp.exe"
    } else {
        "objectiveai-mcp"
    };
    let exe = ctx.filesystem.base_dir().join("bin").join(bin);

    let listening = crate::spawn::spawn_and_wait_for_listening(&exe, &address, port, &[]).await?;
    Ok(Response { listening })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::mcp::spawn as sdk;
    use objectiveai_sdk::cli::command::mcp::spawn::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::mcp::spawn as sdk;
    use objectiveai_sdk::cli::command::mcp::spawn::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
