//! `config functions profiles pairs favorites get` — TODO: SDK
//! `ResponseItem` currently exposes a single `path`, but on-disk
//! `PairFavorite` has separate `function` + `profile` paths. Cannot
//! collapse to one path without losing information. Fill in once the
//! SDK leaf surfaces both paths.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::get::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("SDK ResponseItem single-path mismatch with on-disk PairFavorite (function+profile)")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::get as sdk;
    use objectiveai_sdk::cli::command::config::functions::profiles::pairs::favorites::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
