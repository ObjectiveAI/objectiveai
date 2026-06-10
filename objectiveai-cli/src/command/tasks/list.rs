//! `agents tasks list` — read schedules for one or more `--target`s.
//!
//! Each target resolves to an AIH (`me` / `instance=…` / `tag=…` —
//! BOUND-only; GROUPED / ABSENT error) and its matching schedules are
//! queried concurrently; rows stream out as each target's query lands,
//! deduped by row id. Kind (`oneshot`/`interval`), readiness
//! (`pending`/`exhausted`), `after_id`, and `count` filter each query.

use std::collections::HashSet;
use std::pin::Pin;

use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::tasks::list::{Plugin, Request, ResponseItem};

use crate::context::Context;
use crate::db::tasks::ListedSchedule;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let db = ctx.db.clone();
    let oneshot = request.oneshot;
    let interval = request.interval;
    let pending = request.pending;
    let exhausted = request.exhausted;
    let after_id = request.after_id;
    let count = request.count;
    let stream = async_stream::stream! {
        // Resolve + query every target concurrently.
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let db = db.clone();
            let default_parent = default_parent.clone();
            inflight.push(async move {
                let aih = super::resolve_target(&db, target, &default_parent).await?;
                let rows = crate::db::tasks::list_schedules(
                    &db,
                    std::slice::from_ref(&aih),
                    oneshot,
                    interval,
                    pending,
                    exhausted,
                    after_id,
                    count,
                )
                .await?;
                Ok::<Vec<ListedSchedule>, Error>(rows)
            });
        }
        // Yield each schedule the moment its target's query lands — never
        // waiting for the slowest. A `seen` set dedups by row id across
        // targets that resolve to the same AIH.
        let mut seen = HashSet::new();
        while let Some(result) = inflight.next().await {
            match result {
                Ok(rows) => {
                    for r in rows {
                        if seen.insert(r.id) {
                            yield Ok(to_response_item(r));
                        }
                    }
                }
                Err(e) => yield Err(e),
            }
        }
    };
    Ok(Box::pin(stream))
}

fn to_response_item(r: ListedSchedule) -> ResponseItem {
    ResponseItem {
        id: r.id,
        name: r.name,
        agent_instance_hierarchy: r.agent_instance_hierarchy,
        command: r.command,
        description: r.description,
        created_at: r.created_at,
        last_ran_at: r.last_ran_at,
        interval: r.interval_seconds.map(|secs| {
            humantime::format_duration(std::time::Duration::from_secs(secs)).to_string()
        }),
        version: r.version as u64,
        plugin: r.plugin.map(|p| Plugin {
            owner: p.owner,
            repository: p.repository,
            version: p.version,
        }),
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::request_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::Request),
        ))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::tasks::list as sdk;
    use objectiveai_sdk::cli::command::tasks::list::response_schema::{
        Request, Response,
    };

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(
            schemars::schema_for!(sdk::ResponseItem),
        ))
    }
}
