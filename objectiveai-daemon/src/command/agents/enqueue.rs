//! `agents enqueue` — fire-and-forget into the queue.
//!
//! Bypasses the lock race and spawn child entirely (that's `agents
//! message`); writes one row into `message_queue` against the
//! target. With `--key`, `db::message_queue::enqueue_with_content`
//! replaces any prior active row scoped to the same (target, key)
//! pair before insert, so re-issuing with the same key reliably
//! replaces the prior payload. Refs have no queue identity — error.
//! Tags need NOT exist yet: the row is parked against the tag NAME
//! whether or not the tag is registered, and the queue's two-rule read
//! predicate resolves it once the tag binds to an agent that reads
//! (park-now, deliver-on-bind). A tag that never binds simply leaves
//! the row unconsumed.

use objectiveai_sdk::cli::command::agents::enqueue::{Request, Response};
use objectiveai_sdk::cli::command::agents::selector::AgentSelector;

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let content = super::message::resolve_message(ctx, request.message).await?;
    let (hier, tag) = match request.agent {
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            (Some(format!("{parent}/{agent_instance}")), None)
        }
        // No existence check: park the row against the tag NAME whether
        // or not the tag is registered yet. The queue's read predicate
        // resolves it once the tag binds to a reading agent (park-now,
        // deliver-on-bind).
        AgentSelector::Tag { agent_tag } => (None, Some(agent_tag)),
        AgentSelector::Ref { .. } => return Err(Error::EnqueueRefTarget),
    };
    let id = crate::db::message_queue::enqueue_with_content(
        ctx.db_client().await?,
        hier.clone(),
        tag.clone(),
        &ctx.config.agent_instance_hierarchy,
        request.key,
        content,
    )
    .await?;
    Ok(Response {
        id,
        agent_instance_hierarchy: hier,
        agent_tag: tag,
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::enqueue as sdk;
    use objectiveai_sdk::cli::command::agents::enqueue::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::enqueue as sdk;
    use objectiveai_sdk::cli::command::agents::enqueue::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
