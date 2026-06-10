//! `agents instances get` — fetch per-agent aggregates for the EXACT
//! resolved targets (not their children). An explicitly-named target
//! always yields an item, zero-filled when it has no activity. Backed
//! by `db::instances::get_exact`.

use std::collections::BTreeMap;
use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::instances::get::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();

    // Resolve each target to one exact AIH (GROUPED/ABSENT tags skip),
    // dedup preserving first-seen order.
    let mut aihs: Vec<String> = Vec::new();
    for target in request.targets {
        if let Some(aih) = super::resolve_target(&ctx.db, target, &default_parent).await? {
            if !aihs.contains(&aih) {
                aihs.push(aih);
            }
        }
    }

    // Fetch each agent exactly; BTreeMap sorts + dedups across targets
    // that resolved to the same AIH.
    let mut merged: BTreeMap<String, ResponseItem> = BTreeMap::new();
    for aih in aihs {
        let item = crate::db::instances::get_exact(&ctx.db, &aih).await?;
        merged.insert(item.agent_instance_hierarchy.clone(), item);
    }

    let items: Vec<Result<ResponseItem, Error>> =
        merged.into_values().map(Ok).collect();
    Ok(Box::pin(futures::stream::iter(items)))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::get as sdk;
    use objectiveai_sdk::cli::command::agents::instances::get::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::get as sdk;
    use objectiveai_sdk::cli::command::agents::instances::get::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
