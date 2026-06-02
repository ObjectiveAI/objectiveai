//! `config swarms favorites del` — remove a named entry from the
//! swarm favorites list in on-disk config.

use objectiveai_sdk::cli::command::config::swarms::favorites::del::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    config.swarms().del_favorite(&request.name)?;
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::del as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::del::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::del as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::del::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
