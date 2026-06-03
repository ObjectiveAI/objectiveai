//! `functions get` — read a function definition by remote path or
//! favorite-name reference.

use objectiveai_sdk::cli::command::RemotePathCommitOptionalOrFavorite;
use objectiveai_sdk::cli::command::functions::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = match request.path {
        RemotePathCommitOptionalOrFavorite::Resolved(p) => p,
        RemotePathCommitOptionalOrFavorite::Favorite(name) => {
            let mut config = ctx.filesystem.read_config().await?;
            let fav = config
                .functions()
                .get_favorites()
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            fav.path.clone()
        }
    };
    Ok(objectiveai_sdk::functions::get_function(&ctx.http, path).await?)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::functions::get as sdk;
    use objectiveai_sdk::cli::command::functions::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::functions::get as sdk;
    use objectiveai_sdk::cli::command::functions::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
