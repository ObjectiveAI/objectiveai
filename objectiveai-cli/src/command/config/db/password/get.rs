//! `config db password get` — read `db.password` from on-disk config.

use objectiveai_sdk::cli::command::config::db::password::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    Ok(Response {
        password: config.db().get_password().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::db::password::get as sdk;
    use objectiveai_sdk::cli::command::config::db::password::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::db::password::get as sdk;
    use objectiveai_sdk::cli::command::config::db::password::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
