//! `config swarms favorites get` — stream every saved swarm favorite,
//! one `ResponseItem` per record.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::config::swarms::favorites::get::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, _request: Request) -> Result<ItemStream, Error> {
    let mut config = ctx.filesystem.read_config().await?;
    let items: Vec<ResponseItem> = config
        .swarms()
        .get_favorites()
        .iter()
        .map(|f| ResponseItem {
            name: f.get_name().to_string(),
            path: f.path.clone(),
            note: f.get_note().to_string(),
        })
        .collect();
    Ok(Box::pin(futures::stream::iter(items.into_iter().map(Ok))))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::get as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Request))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::config::swarms::favorites::get as sdk;
    use objectiveai_sdk::cli::command::config::swarms::favorites::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(schemars::schema_for!(sdk::Response))
    }
}
