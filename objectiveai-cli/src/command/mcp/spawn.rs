//! `mcp spawn` — start the `objectiveai-mcp` server in the background,
//! using `mcp.address` + `mcp.port` from on-disk config (mcp is the
//! one tier that kept a configurable bind address).
//!
//! The mcp server is per-state: its lock lives at
//! `<dir>/state/<state>/locks` key `mcp`, and the lock contents are
//! the server's client-connect URL. If the lock is already held the
//! server is already up and its published URL is returned as-is.

use objectiveai_sdk::cli::command::mcp::spawn::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx
        .filesystem
        .read_config_view(objectiveai_sdk::cli::command::GetScope::Final)
        .await?;
    let address = config.mcp().get_address().map(String::from);
    let port = config.mcp().get_port();

    let bin = if cfg!(windows) {
        "objectiveai-mcp.exe"
    } else {
        "objectiveai-mcp"
    };
    let exe = ctx.filesystem.bin_dir().join(bin);
    let lock_dir = ctx.filesystem.state_dir().join("locks");

    // Fresh env: layout coordinates + the configured bind address and
    // port, each omitted when unset so the server falls back to its
    // own defaults.
    let listening = crate::spawn::spawn_until_lock_published(&exe, &lock_dir, "mcp", |cmd| {
        cmd.env("OBJECTIVEAI_DIR", ctx.filesystem.dir())
            .env("OBJECTIVEAI_STATE", ctx.filesystem.state())
            .env("SUPPRESS_OUTPUT", "true");
        if let Some(address) = address {
            cmd.env("ADDRESS", address);
        }
        if let Some(port) = port {
            cmd.env("PORT", port.to_string());
        }
    })
    .await?;
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
