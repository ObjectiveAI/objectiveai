//! `config functions inventions remote set` — write a new `Remote`
//! value (`github` | `filesystem`) into the function-inventions section
//! of on-disk config. `mock` is rejected by `set_remote`.

use objectiveai_sdk::Remote;
use objectiveai_sdk::cli::command::config::functions::inventions::remote::set::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let remote = match request.value.as_str() {
        "github" => Remote::Github,
        "filesystem" => Remote::Filesystem,
        "mock" => Remote::Mock,
        other => return Err(Error::PathParse(format!("invalid remote: {other}"))),
    };
    let mut config = ctx.filesystem.read_config().await?;
    config.functions().inventions().set_remote(remote)?;
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::set as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::set::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::set as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::set::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
