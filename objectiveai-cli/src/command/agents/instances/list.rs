//! `agents instances list` — enumerate the DIRECT children of one or
//! more resolved targets, with per-agent aggregates (tags, queued
//! count, spawn/active timestamps, total logged messages). Backed by
//! `db::instances::list_under_parent`.

use std::collections::BTreeMap;
use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::instances::list::{Request, ResponseItem};

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();

    // Resolve each target to a parent AIH whose direct children get
    // listed. GROUPED/ABSENT tags resolve to None and are skipped.
    let mut parents: Vec<String> = Vec::new();
    for target in request.targets {
        if let Some(parent) =
            super::resolve_target(&ctx.db, target, &default_parent).await?
        {
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
    }

    // Query each parent's direct children and merge by AIH. The
    // BTreeMap keeps the output sorted and deduped across
    // overlapping/nested targets.
    let mut merged: BTreeMap<String, ResponseItem> = BTreeMap::new();
    for parent in parents {
        let items = crate::db::instances::list_under_parent(&ctx.db, &parent).await?;
        for item in items {
            merged.insert(item.agent_instance_hierarchy.clone(), item);
        }
    }

    let items: Vec<Result<ResponseItem, Error>> =
        merged.into_values().map(Ok).collect();
    Ok(Box::pin(futures::stream::iter(items)))
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::list as sdk;
    use objectiveai_sdk::cli::command::agents::instances::list::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::list as sdk;
    use objectiveai_sdk::cli::command::agents::instances::list::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
