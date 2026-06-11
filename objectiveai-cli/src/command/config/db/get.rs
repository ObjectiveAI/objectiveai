//! `config db get` — read the db section of on-disk config (address
//! + port + user + password + database). Missing fields stay `None`.

use objectiveai_sdk::cli::command::config::db::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let db = config.db();
    Ok(Response {
        address: db.get_address().map(String::from),
        port: db.get_port(),
        user: db.get_user().map(String::from),
        password: db.get_password().map(String::from),
        database: db.get_database().map(String::from),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::db::get as sdk;
    use objectiveai_sdk::cli::command::config::db::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::db::get as sdk;
    use objectiveai_sdk::cli::command::config::db::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
