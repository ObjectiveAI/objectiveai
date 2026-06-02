//! `config functions inventions get` — read the function-inventions
//! section of on-disk config (currently a single `Remote` field).

use objectiveai_sdk::cli::command::config::functions::inventions::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let remote = config.functions().inventions().get_remote();
    Ok(Response { remote })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
