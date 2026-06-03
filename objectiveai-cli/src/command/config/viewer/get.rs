//! `config viewer get` — read the viewer section of on-disk config
//! (address + port + secret + signature). Missing fields stay `None`.

use objectiveai_sdk::cli::command::config::viewer::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let viewer = config.viewer();
    Ok(Response {
        address: viewer.get_address().map(String::from),
        port: viewer.get_port(),
        secret: viewer.get_secret().map(String::from),
        signature: viewer.get_signature().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::viewer::get as sdk;
    use objectiveai_sdk::cli::command::config::viewer::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::viewer::get as sdk;
    use objectiveai_sdk::cli::command::config::viewer::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
