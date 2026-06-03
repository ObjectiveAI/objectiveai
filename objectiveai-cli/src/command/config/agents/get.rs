//! `config agents get` — read the agents section of on-disk config.
//! Today the only field is `favorites`; the `filter` field on the
//! Request is reserved for future jq-style projection and is
//! ignored here (jq application has moved to `run.rs`).

use objectiveai_sdk::cli::command::config::agents::favorites::get::ResponseItem;
use objectiveai_sdk::cli::command::config::agents::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, _request: Request) -> Result<Response, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let favorites: Vec<ResponseItem> = config
        .agents()
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
    use objectiveai_sdk::cli::command::config::agents::get as sdk;
    use objectiveai_sdk::cli::command::config::agents::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::agents::get as sdk;
    use objectiveai_sdk::cli::command::config::agents::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
