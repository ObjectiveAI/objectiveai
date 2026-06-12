//! `config db database get` — read `db.database` from on-disk config.

use objectiveai_sdk::cli::command::db::config::database::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config_view(request.scope).await?;
    Ok(Response {
        database: config.db().get_database().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::db::config::database::get as sdk;
    use objectiveai_sdk::cli::command::db::config::database::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::db::config::database::get as sdk;
    use objectiveai_sdk::cli::command::db::config::database::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
