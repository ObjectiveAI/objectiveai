//! `config functions inventions remote get` — read the function-
//! inventions `Remote` from on-disk config.

use objectiveai_sdk::cli::command::config::functions::inventions::remote::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(config.functions().inventions().get_remote())
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::inventions::remote::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
