//! `update` — bare-naked streaming handler. Refreshes all four shipped
//! binaries from the latest GitHub release, emitting one `ResponseItem`
//! per (asset, stage) pair as the update progresses.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::update::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(_ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    todo!("update execute — port from src/updater.rs (legacy emits Notification::Updater events through Handle; rewrite to emit ResponseItem events through a channel-backed Stream)")
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::update as sdk;
    use objectiveai_sdk::cli::command::update::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::update as sdk;
    use objectiveai_sdk::cli::command::update::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
