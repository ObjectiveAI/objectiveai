//! `swarms get <path>` — fetch a swarm by its remote path or by a
//! favorite name. The SDK's `TryFrom<Args>` has already parsed the
//! docker-style `key=value,...` string into a
//! `RemotePathCommitOptionalOrFavorite`; here we just dispatch on the
//! variant — `Resolved` short-circuits straight to `get_swarm`,
//! `Favorite` reads on-disk swarm favorites to look up the name.

use objectiveai_sdk::cli::command::RemotePathCommitOptionalOrFavorite;
use objectiveai_sdk::cli::command::swarms::get::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let path = match request.path {
        RemotePathCommitOptionalOrFavorite::Resolved(p) => p,
        RemotePathCommitOptionalOrFavorite::Favorite(name) => {
            let config = ctx.filesystem.read_config().await?;
            let fav = config
                .swarms()
                .get_favorites()
                .iter()
                .find(|f| f.get_name() == name)
                .ok_or_else(|| Error::FavoriteNotFound(name.clone()))?;
            fav.path.clone()
        }
    };
    let response = objectiveai_sdk::swarm::get_swarm(&ctx.http, path).await?;
    Ok(response)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::swarms::get as sdk;
    use objectiveai_sdk::cli::command::swarms::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
