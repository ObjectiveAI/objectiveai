//! `config functions profiles get` — read the function-profiles
//! section of on-disk config.

use objectiveai_sdk::cli::command::config::functions::profiles::favorites::get::ResponseItem;
use objectiveai_sdk::cli::command::config::functions::profiles::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let favorites: Vec<ResponseItem> = config
        .functions()
        .profiles()
        .get_favorites()
        .iter()
        .map(|f| ResponseItem {
            name: f.get_name().to_string(),
            path: f.path.clone(),
            note: f.get_note().to_string(),
        })
        .collect();
    Ok(Response {
        favorites: Some(favorites),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
