//! `config functions profiles pairs favorites add` — add a named entry
//! to the function-profiles pair-favorites list in on-disk config.

use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;
use crate::filesystem::config::PairFavorite;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let favorite =
        PairFavorite::new(request.name, request.function, request.profile, request.note)?;
    let mut config = ctx.filesystem.read_config().await?;
    config.functions().profiles().pairs().add_favorite(favorite);
    ctx.filesystem.write_config(&config).await?;
    Ok(Response::Ok)
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
