//! `agents wait` — block until an agent is done.
//!
//! Targets an instance hierarchy or a tag (a plain ref has no live
//! identity — error). The whole wait, across BOTH subscriptions on
//! the tag route, runs under the request's humantime timeout.
//!
//! - **Instance**: subscribe to the AIH lock's release
//!   ([`objectiveai_sdk::lockfile::wait_released`] returns
//!   immediately when nobody holds it).
//! - **BOUND tag**: resolve to its hierarchy, then the instance wait.
//! - **GROUPED (un-upgraded) tag**: the tag lock's holder is the
//!   spawn materializing the tag. Nobody holding it ⇒ nothing is
//!   materializing it ⇒ done (re-checked against the DB first — a
//!   racer may have upgraded+released between our lookup and the
//!   probe). Held ⇒ wait for release, then re-resolve: the spawn
//!   flow commits the GROUPED→BOUND upgrade strictly BEFORE
//!   releasing the tag lock, so a still-GROUPED tag here is a
//!   systemic invariant violation and errors fatally; the freshly
//!   bound hierarchy falls through to the instance wait.

use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::wait::{Request, Response};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let timeout_seconds = request
        .base
        .timeout_seconds
        .ok_or(Error::MissingRequestField { field: "timeout_seconds" })?;
    tokio::time::timeout(
        std::time::Duration::from_secs(timeout_seconds),
        wait(ctx, request.agent),
    )
    .await
    .map_err(|_| Error::AgentWaitTimeout { timeout_seconds })?
}

async fn wait(ctx: &Context, agent: AgentSelector) -> Result<Response, Error> {
    let state_dir = ctx.filesystem.state_dir();

    let hierarchy = match agent {
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            format!("{parent}/{agent_instance}")
        }
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                } => agent_instance_hierarchy,
                crate::db::tags::LookupState::Grouped { .. } => {
                    match wait_for_tag_upgrade(ctx, &state_dir, agent_tag).await? {
                        Some(agent_instance_hierarchy) => agent_instance_hierarchy,
                        // Nothing is materializing the tag — done.
                        None => return Ok(Response::Ok),
                    }
                }
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
        AgentSelector::Ref { .. } => return Err(Error::WaitRefTarget),
    };

    let (dir, key) = super::locks::agent_instance_lock(&state_dir, &hierarchy);
    objectiveai_sdk::lockfile::wait_released(&dir, &key)
        .await
        .map_err(|source| Error::Lockfile { key, source })?;
    Ok(Response::Ok)
}

/// GROUPED-tag arm: wait out the materializing spawn (if any) and
/// return the hierarchy the tag got bound to — `None` when nothing
/// holds the tag lock and the DB still says GROUPED (no spawn in
/// flight, nothing to wait for).
async fn wait_for_tag_upgrade(
    ctx: &Context,
    state_dir: &std::path::Path,
    agent_tag: String,
) -> Result<Option<String>, Error> {
    let (dir, key) = super::locks::agent_tag_lock(state_dir, &agent_tag);

    if objectiveai_sdk::lockfile::try_held(&dir, &key).await {
        objectiveai_sdk::lockfile::wait_released(&dir, &key)
            .await
            .map_err(|source| Error::Lockfile { key, source })?;
        // The spawn flow upgrades GROUPED→BOUND strictly before
        // releasing the tag lock — a still-GROUPED tag here means
        // that invariant is broken somewhere.
        match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
            crate::db::tags::LookupState::Bound {
                agent_instance_hierarchy,
            } => Ok(Some(agent_instance_hierarchy)),
            crate::db::tags::LookupState::Grouped { .. } => {
                Err(Error::TagLockDroppedWithoutUpgrade { tag: agent_tag })
            }
            crate::db::tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        }
    } else {
        // Unlocked. Re-check the DB before concluding "idle": a
        // racer may have upgraded AND released between the caller's
        // lookup and our probe.
        match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
            crate::db::tags::LookupState::Bound {
                agent_instance_hierarchy,
            } => Ok(Some(agent_instance_hierarchy)),
            crate::db::tags::LookupState::Grouped { .. } => Ok(None),
            crate::db::tags::LookupState::Absent => Err(Error::TagNotFound(agent_tag)),
        }
    }
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::wait as sdk;
    use objectiveai_sdk::cli::command::agents::wait::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::wait as sdk;
    use objectiveai_sdk::cli::command::agents::wait::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
