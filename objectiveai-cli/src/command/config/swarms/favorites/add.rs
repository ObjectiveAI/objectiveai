//! `config swarms favorites add` — add a named entry to the swarm
//! favorites list in on-disk config.

use objectiveai_sdk::cli::command::config::swarms::favorites::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::Favorite;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let favorite = Favorite::new(request.name, request.path, request.note)?;
    let mut config = ctx.filesystem.read_config().await?;
    config.swarms().add_favorite(favorite);
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
