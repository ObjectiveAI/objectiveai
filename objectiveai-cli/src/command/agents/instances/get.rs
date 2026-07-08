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
        if let Some(aih) = super::resolve_target(ctx.db_client().await?, target, &default_parent).await? {
            if !aihs.contains(&aih) {
                aihs.push(aih);
            }
        }
    }

    // Fetch each agent exactly; BTreeMap sorts + dedups across targets
    // that resolved to the same AIH.
    let mut merged: BTreeMap<String, ResponseItem> = BTreeMap::new();
    for aih in aihs {
        let mut item = crate::db::instances::get_exact(ctx.db_client().await?, &aih).await?;
        // The recorded definition source: agent_refs, with the
        // legacy request-blob fallback. None when neither knows the
        // agent.
        item.agent = crate::db::logs::lookup_session(ctx.db_client().await?, &aih)
            .await?
            .map(|lookup| lookup.agent);
        // The effective laboratory set the next spawn pass dials:
        // the AIH's own attachments UNION its bound tags'.
        item.laboratories = Some(
            crate::db::laboratory_attachments::effective_for_aih(
                ctx.db_client().await?,
                &aih,
                &item.tags,
            )
            .await?
            .into_iter()
            .map(|record| {
                objectiveai_sdk::cli::command::agents::instances::list::LaboratoryAttachment {
                    id: record.laboratory_id,
                    attached_at: crate::db::time::unix_to_rfc3339(record.attached_at),
                    attached_by: record.attached_by,
                }
            })
            .collect(),
        );
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
