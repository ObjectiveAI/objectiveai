//! `agents read all` — bare-naked streaming handler stub.

use std::pin::Pin;

use futures::Stream;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use objectiveai_sdk::cli::command::agents::instances::read::all::{Request, ResponseItem, Target};

use crate::context::Context;
use crate::db::tags;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let db = ctx.db.clone();
    let stream = async_stream::stream! {
        let mut inflight = FuturesUnordered::new();
        for target in request.targets {
            let db = db.clone();
            let default_parent = default_parent.clone();
            inflight.push(async move {
                let (_parent, _spawned, _leaf) =
                    resolve_target(&db, target, &default_parent).await?;
                // Reader endpoints will return real items once the
                // postgres-backed `logs.*` reader lands. Until then
                // the leaf shape stays compatible: error out as
                // NotImplemented so callers see a structured signal
                // instead of silently empty results.
                Err::<ResponseItem, _>(Error::NotImplemented(
                    "agents instances read all (postgres reader pending)",
                ))
            });
        }
        while let Some(result) = inflight.next().await {
            yield result;
        }
    };
    Ok(Box::pin(stream))
}

/// Resolve one `Target` to a `(parent, spawned, leaf)` triple. Direct
/// mode uses the explicit `parent=` if any, otherwise the cli's own
/// `Config.agent_instance_hierarchy`. Tag mode looks the tag up via
/// the postgres-backed `tags` tier and errors out on PENDING / ABSENT.
async fn resolve_target(
    db: &crate::db::Pool,
    target: Target,
    default_parent: &str,
) -> Result<(String, String, String), Error> {
    match target {
        Target::Direct {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent =
                parent_agent_instance_hierarchy.unwrap_or_else(|| default_parent.to_string());
            let spawned = format!("{parent}/{agent_instance}");
            Ok((parent, spawned, agent_instance))
        }
        Target::Tag { agent_tag } => match tags::lookup(db, &agent_tag).await? {
            tags::LookupState::Bound { agent_instance_hierarchy } => {
                let parent = tags::parent_of(&agent_instance_hierarchy).to_string();
                let leaf = tags::leaf_of(&agent_instance_hierarchy).to_string();
                Ok((parent, agent_instance_hierarchy, leaf))
            }
            tags::LookupState::Pending {
                parent_agent_instance_hierarchy,
                agent_full_id,
            } => Err(Error::TagPending {
                tag: agent_tag,
                parent_agent_instance_hierarchy,
                agent_full_id,
            }),
            tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        },
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::all as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::all::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::all as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::all::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
