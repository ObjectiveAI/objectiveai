//! `config functions profiles pairs favorites add` — TODO: SDK
//! Request exposes a single `path`, but on-disk `PairFavorite`
//! requires both `function` and `profile` paths. Fill in once the SDK
//! Request surfaces both.

use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
    todo!("SDK Request single-path mismatch with on-disk PairFavorite (function+profile)")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::add::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
