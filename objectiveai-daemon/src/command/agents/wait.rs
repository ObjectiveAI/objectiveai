//! `agents wait` — block until an agent reaches the REQUESTED status
//! (`request.active`: `false` = done/inactive, the original behavior;
//! `true` = up/active). Uncapped either way; a target already in the
//! requested status returns immediately.
//!
//! Targets an instance hierarchy or a tag (a plain ref has no live
//! identity — error).
//!
//! **Inactive** (`active: false`):
//! - **Instance**: subscribe to the AIH lock's release
//!   ([`crate::command::agents::locks::wait_released`] returns
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
//!
//! **Active** (`active: true`): the AIH lock map only signals
//! RELEASES, so the acquire edge comes from the live registry —
//! [`crate::http::agents_routes::ActiveAgents`]'s `Activated`
//! broadcast, fired right after an agent wins its instance lock. The
//! ordering is SUBSCRIBE FIRST, PROBE SECOND: `activate` emits after
//! the lock acquire, so an acquire that beats the probe is seen by
//! the probe ([`crate::command::agents::locks::try_held`]) and one
//! after it lands in the subscription — no missed edge.
//! - **Instance**: probe the AIH lock; held ⇒ done now, else await
//!   its `Activated`.
//! - **BOUND tag**: its hierarchy, then the instance wait.
//! - **GROUPED tag**: first await the tag BINDING (re-`lookup` on
//!   every `TagsChanged` broadcast), then the instance wait on the
//!   bound hierarchy.

use objectiveai_sdk::cli::command::agents::selector::AgentSelector;
use objectiveai_sdk::cli::command::agents::wait::{Request, Response};

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    if request.active {
        wait_active(global, scoped, request.agent).await
    } else {
        wait(global, scoped, request.agent).await
    }
}

/// The `--active` direction: resolve when the target holds its
/// instance lock. See the module docs for the subscribe-before-probe
/// ordering rationale.
async fn wait_active(
    global: &GlobalContext, scoped: &ScopedContext,
    agent: AgentSelector,
) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Instance("agents wait --active requires the resident daemon".to_string())
    })?;
    let state_dir = scoped.filesystem.state_dir();
    // Subscribe BEFORE any probe or lookup — every edge from here on
    // is either already visible to the probe or lands in this
    // receiver.
    let mut rx = hubs.active.subscribe();

    let hierarchy = match agent {
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(scoped.agent_instance_hierarchy());
            format!("{parent}/{agent_instance}")
        }
        AgentSelector::Tag { agent_tag } => {
            // Re-`lookup` until BOUND, advancing on `TagsChanged`
            // broadcasts (a GROUPED tag binds when a spawn
            // materializes it — that flip fires the tags NOTIFY).
            loop {
                match crate::db::tags::lookup(&global.db_client().await?, &agent_tag).await? {
                    crate::db::tags::LookupState::Bound {
                        agent_instance_hierarchy,
                    } => break agent_instance_hierarchy,
                    crate::db::tags::LookupState::Grouped { .. } => {
                        use crate::http::agents_routes::StatusChange;
                        loop {
                            match rx.recv().await {
                                Ok(StatusChange::TagsChanged { .. }) => break,
                                // A lagged receiver may have missed a
                                // TagsChanged — re-lookup regardless.
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    break;
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    return Err(Error::Instance(
                                        "agents wait --active: live registry closed"
                                            .to_string(),
                                    ));
                                }
                                Ok(_) => continue,
                            }
                        }
                    }
                    crate::db::tags::LookupState::Absent => {
                        return Err(Error::TagNotFound(agent_tag));
                    }
                }
            }
        }
        AgentSelector::Ref { .. } => return Err(Error::WaitRefTarget),
    };

    let (dir, key) = super::locks::agent_instance_lock(&state_dir, &hierarchy);
    if super::locks::try_held(global.agent_locks(), &dir, &key) {
        return Ok(Response::Ok);
    }
    loop {
        use crate::http::agents_routes::StatusChange;
        match rx.recv().await {
            Ok(StatusChange::Activated {
                agent_instance_hierarchy,
            }) if agent_instance_hierarchy == hierarchy => return Ok(Response::Ok),
            // Missed events — fall back to the probe before resuming.
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                if super::locks::try_held(global.agent_locks(), &dir, &key) {
                    return Ok(Response::Ok);
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                return Err(Error::Instance(
                    "agents wait --active: live registry closed".to_string(),
                ));
            }
            Ok(_) => continue,
        }
    }
}

async fn wait(global: &GlobalContext, scoped: &ScopedContext, agent: AgentSelector) -> Result<Response, Error> {
    let state_dir = scoped.filesystem.state_dir();

    let hierarchy = match agent {
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(scoped.agent_instance_hierarchy());
            format!("{parent}/{agent_instance}")
        }
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(&global.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                } => agent_instance_hierarchy,
                crate::db::tags::LookupState::Grouped { .. } => {
                    match wait_for_tag_upgrade(global, scoped, &state_dir, agent_tag).await? {
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
    super::locks::wait_released(global.agent_locks(), &dir, &key).await;
    Ok(Response::Ok)
}

/// GROUPED-tag arm: wait out the materializing spawn (if any) and
/// return the hierarchy the tag got bound to — `None` when nothing
/// holds the tag lock and the DB still says GROUPED (no spawn in
/// flight, nothing to wait for).
async fn wait_for_tag_upgrade(
    global: &GlobalContext, _scoped: &ScopedContext,
    state_dir: &std::path::Path,
    agent_tag: String,
) -> Result<Option<String>, Error> {
    let (dir, key) = super::locks::agent_tag_lock(state_dir, &agent_tag);

    if super::locks::try_held(global.agent_locks(), &dir, &key) {
        super::locks::wait_released(global.agent_locks(), &dir, &key).await;
        // The spawn flow upgrades GROUPED→BOUND strictly before
        // releasing the tag lock — a still-GROUPED tag here means
        // that invariant is broken somewhere.
        match crate::db::tags::lookup(&global.db_client().await?, &agent_tag).await? {
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
        match crate::db::tags::lookup(&global.db_client().await?, &agent_tag).await? {
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

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::wait as sdk;
    use objectiveai_sdk::cli::command::agents::wait::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
