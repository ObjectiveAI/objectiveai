//! `agents read subscribe` — stub. The Unix-socket event stream that
//! backed live `subscribe` is gone; the postgres-backed reader hasn't
//! landed yet. Until it does, the handler returns the structured
//! `NotImplemented` signal so callers see a typed error instead of a
//! silently empty stream.

use std::pin::Pin;

use futures::Stream;
use objectiveai_sdk::cli::command::agents::instances::read::subscribe::{
    Request, ResponseItem, SubscribeTarget,
};
use tokio::sync::mpsc;

use crate::context::Context;
use crate::error::Error;

type ItemStream = Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>;

pub async fn execute(ctx: &Context, request: Request) -> Result<ItemStream, Error> {
    let db = ctx.db.clone();
    let default_parent = ctx.config.agent_instance_hierarchy.clone();
    let (tx, rx) = mpsc::channel::<Result<ResponseItem, Error>>(16);
    tokio::spawn(async move {
        // Resolve the target so callers see TagPending / TagNotFound /
        // ABSENT errors even while reads are stubbed.
        let resolved = match request.target {
            SubscribeTarget::Direct {
                parent_agent_instance_hierarchy,
                agent_instance,
            } => {
                let parent =
                    parent_agent_instance_hierarchy.unwrap_or_else(|| default_parent.clone());
                let spawned = format!("{parent}/{agent_instance}");
                Ok((parent, spawned, agent_instance))
            }
            SubscribeTarget::Tag { agent_tag } => resolve_tag(&db, agent_tag).await,
        };
        let send = match resolved {
            Ok(_) => Err(Error::NotImplemented(
                "agents instances read subscribe (postgres reader pending)",
            )),
            Err(e) => Err(e),
        };
        let _ = tx.send(send).await;
    });
    Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
}

/// Resolve a `--agent-tag` to the `(parent, spawned, leaf)` triple
/// the rest of the handler expects. BOUND tags split into parent +
/// leaf via [`crate::db::tags::parent_of`] / [`leaf_of`]; PENDING and
/// ABSENT both raise structured errors so the caller sees why the
/// lookup failed.
async fn resolve_tag(
    db: &crate::db::Pool,
    agent_tag: String,
) -> Result<(String, String, String), Error> {
    use crate::db::tags;
    match tags::lookup(db, &agent_tag).await? {
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
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe as sdk;
    use objectiveai_sdk::cli::command::agents::instances::read::subscribe::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::ResponseItem)))
    }
}
