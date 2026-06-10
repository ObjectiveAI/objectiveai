//! `agents instances list` — enumerate agent instances that are
//! descendants of one or more resolved targets, with per-agent
//! aggregates (tags, queued count, spawn/active timestamps, total
//! logged messages). Backed by `db::instances::list_under_parent`.

use std::collections::BTreeMap;
use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::instances::list::{Request, ResponseItem, Target};

use crate::context::Context;
use crate::db::tags;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();

    // Resolve each target to a parent AIH whose descendants get listed.
    // GROUPED/ABSENT tags resolve to None and are silently skipped.
    let mut parents: Vec<String> = Vec::new();
    for target in request.targets {
        if let Some(parent) =
            resolve_target_to_parent(&ctx.db, target, &default_parent).await?
        {
            if !parents.contains(&parent) {
                parents.push(parent);
            }
        }
    }

    // Query each parent's descendants and merge by AIH. The BTreeMap
    // keeps the output sorted and deduped across overlapping/nested
    // targets.
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

/// Resolve a `Target` to the parent AIH whose descendants get listed.
/// `Me`/`Direct` always resolve; a `Tag` resolves only when BOUND —
/// GROUPED/ABSENT yield `None` (the target is silently skipped).
async fn resolve_target_to_parent(
    db: &crate::db::Pool,
    target: Target,
    default_parent: &str,
) -> Result<Option<String>, Error> {
    match target {
        Target::Me => Ok(Some(default_parent.to_string())),
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .unwrap_or_else(|| default_parent.to_string());
            Ok(Some(format!("{parent}/{agent_instance}")))
        }
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound {
                agent_instance_hierarchy,
            } => Ok(Some(agent_instance_hierarchy)),
            tags::LookupState::Grouped { .. } | tags::LookupState::Absent => Ok(None),
        },
    }
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
