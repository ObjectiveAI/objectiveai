//! `functions inventions state get` — read an invention state by
//! docker-style path filter. The `filter` arg parses as
//! [`crate::favorite_ref::FavoriteRef`], so `favorite=<name>` resolves
//! against `config.functions().get_favorites()`.

use objectiveai_sdk::cli::command::functions::inventions::state::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::favorite_ref::FavoriteRef;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let filter = request
        .filter
        .ok_or(Error::MissingArgs("filter is required"))?;
    let favorite_ref: FavoriteRef = filter.parse().map_err(Error::PathParse)?;
    let fs = ctx.filesystem.clone();
    let path = favorite_ref
        .resolve(|| async move {
            let mut config = match fs.read_config().await {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };
            config.functions().get_favorites().to_vec()
        })
        .await?;
    Ok(
        objectiveai_sdk::functions::inventions::state::get_function_invention_state(
            &ctx.http, path,
        )
        .await?,
    )
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::inventions::state::get as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::state::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::inventions::state::get as sdk;
    use objectiveai_sdk::cli::command::functions::inventions::state::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
