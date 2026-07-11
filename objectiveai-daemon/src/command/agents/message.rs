//! `agents message` — unary delivery primitive with a total FIFO
//! order.
//!
//! The agent input is the shared [`AgentSelector`] — the same shape
//! `agents spawn` takes, so an unspawned agent can be messaged:
//!
//! - **ref**: nothing to lock or enqueue against (no AIH or tag
//!   exists before the spawn) — exec a detached `agents spawn` child
//!   (stream=true) carrying the resolved agent + message inline,
//!   return its first item as `Id`. The only route where content
//!   rides inline.
//! - **instance / BOUND tag** (the AIH lock) and **GROUPED tag**
//!   (the tag lock): ALWAYS enqueue first — the queue row's id fixes
//!   the message's delivery position at commit time — then loop on
//!   the agent's whole lock family (all-or-nothing):
//!     - **held by a live owner** (spawned AFTER our row committed) →
//!       it will drain our row; race the pinned `subscribe_delivered`
//!       (our row flipped inactive → `Delivered`) against
//!       `wait_released` (owner exited → re-race).
//!     - **free — we won it → the agent is IDLE**. Nobody delivers
//!       unless we wake it, so RELEASE the family (we never hand this
//!       OS lock to the child: cross-process lock transfer is unsound
//!       on Windows, and co-holding one lock is impossible) and spawn
//!       a WAKE-UP child (EMPTY message — its startup snapshot and
//!       pass-boundary reads drain the queue oldest-id-first). The
//!       child competes for its OWN family lock; **losing is safe**
//!       (whoever wins drains every pending row). Race `delivered`
//!       against the spawn: `Ok` → the child is delivering, loop and
//!       wait it out; `Err` → an authoritative re-acquire distinguishes
//!       a lost race (someone else runs it → they deliver) from a
//!       genuine failure (row stays parked, durable).
//!
//!   This command never deletes its row — durable across crashes of
//!   any participant, recoverable by a later `message` /
//!   `agents queue deliver`.
//!
//! FIFO: every message is a queue row and every wake spawn starts
//! empty, so delivery order is exactly queue-id (enqueue-commit)
//! order under any interleaving of senders, lock winners, and
//! respawns.
//!
//! Termination is structural, not capped: an agent runs only when a
//! child wins its own family lock (cross-process exclusive), and a
//! child spawned after our row committed always drains it — so after
//! finitely many pre-existing agents, delivery is guaranteed. KNOWN
//! LIMITATION: an agent that emits its `Id` then crashes BEFORE
//! draining our row would be re-woken each exit (a crash-loop of real
//! agent lifetimes, not a tight spin); it needs a deterministically
//! failing agent, since delivery normally precedes `Id`.
//!
//! The message payload is resolved ONCE in this process (file IO /
//! Python run here). Fire-and-forget parking without the race lives
//! in `agents enqueue`.

use futures::StreamExt;
use objectiveai_sdk::agent::completions::message::RichContent;
use objectiveai_sdk::cli::command::agents::message::{Request, RequestMessage, Response};
use objectiveai_sdk::cli::command::agents::selector::{AgentRef, AgentSelector};
use objectiveai_sdk::cli::command::agents::spawn as spawn_sdk;
use objectiveai_sdk::cli::command::{BinaryExecutor, CommandExecutor};

use crate::context::Context;
use crate::error::Error;

pub async fn execute(ctx: &Context, request: Request) -> Result<Response, Error> {
    let Request {
        agent,
        message,
        dangerous_advanced,
        ..
    } = request;
    let seed = dangerous_advanced.as_ref().and_then(|a| a.seed);

    // Resolve the payload once, in this process.
    let content = resolve_message(ctx, message).await?;

    let state_dir = ctx.filesystem.state_dir();
    let route = match agent {
        AgentSelector::Ref { agent } => {
            // Resolve file/python refs HERE too — the child gets the
            // typed agent inline and never re-runs the Python.
            let resolved = super::spawn::resolve_agent_ref(ctx, agent).await?;
            Route::Ref {
                child: AgentSelector::Ref {
                    agent: AgentRef::Resolved(resolved),
                },
            }
        }
        AgentSelector::Instance {
            parent_agent_instance_hierarchy,
            agent_instance,
        } => {
            let parent = parent_agent_instance_hierarchy
                .as_deref()
                .unwrap_or(&ctx.config.agent_instance_hierarchy);
            instance_route(&state_dir, format!("{parent}/{agent_instance}"))
        }
        AgentSelector::Tag { agent_tag } => {
            match crate::db::tags::lookup(ctx.db_client().await?, &agent_tag).await? {
                crate::db::tags::LookupState::Bound {
                    agent_instance_hierarchy,
                } => instance_route(&state_dir, agent_instance_hierarchy),
                crate::db::tags::LookupState::Grouped { tag_group_id, .. } => {
                    let (dir, key) = super::locks::agent_tag_lock(&state_dir, &agent_tag);
                    Route::Locked {
                        dir,
                        key,
                        family: super::locks::Family::Group(tag_group_id),
                        hierarchy: None,
                        tag: Some(agent_tag.clone()),
                        child: AgentSelector::Tag { agent_tag },
                    }
                }
                crate::db::tags::LookupState::Absent => {
                    return Err(Error::TagNotFound(agent_tag));
                }
            }
        }
    };

    match route {
        Route::Ref { child } => spawn_child(child, content, seed).await,
        Route::Locked {
            dir,
            key,
            family,
            hierarchy,
            tag,
            child,
        } => {
            // Park the message FIRST — the row's id fixes its delivery
            // position at commit time, before any lock race. Rows are
            // consumed strictly oldest-id-first, so whoever wins any
            // race below, delivery order stays enqueue order.
            let queue_id = crate::db::message_queue::enqueue_with_content(
                ctx.db_client().await?,
                hierarchy,
                tag,
                &ctx.config.agent_instance_hierarchy,
                None,
                content,
            )
            .await?;
            let pool = ctx.db_client().await?.clone();
            // ONE delivery subscription, pinned OUTSIDE the loop so its
            // LISTEN + probe persist across iterations. Recreating it per
            // iteration let a hot `wait_released` starve it — the mechanism
            // behind the phantom-respawn incident.
            let delivered =
                crate::db::message_queue::subscribe_delivered(&pool, queue_id);
            tokio::pin!(delivered);
            loop {
                // FETCH the current family + LOCK it (all-or-nothing) — the
                // membership can shift during a long wait, so resolve it at
                // lock time. This acquire is BOTH the idle/active probe and
                // (on a win) our brief hold; we never hand this OS lock to
                // the child (cross-process lock transfer is unsound on
                // Windows — the child competes for its own instead).
                match super::locks::try_acquire_family(
                    ctx.agent_locks(),
                    ctx.db_client().await?,
                    &state_dir,
                    family.clone(),
                )
                .await?
                {
                    // ACTIVE: a live owner (spawned AFTER our row committed)
                    // holds the family and will drain our row, or it exits
                    // and we re-race. NEVER spawn here.
                    None => {
                        tokio::select! {
                            biased;
                            delivery = &mut delivered => {
                                delivery?;
                                return Ok(Response::Delivered);
                            }
                            released = objectiveai_sdk::lockfile::wait_released(&dir, &key) => {
                                released.map_err(|e| Error::Lockfile {
                                    key: key.clone(),
                                    source: e,
                                })?;
                            }
                        }
                    }
                    // IDLE: we hold the family. Nobody delivers unless we
                    // wake the agent. RELEASE first — the child is a separate
                    // process and co-holding the same lock is impossible — then
                    // spawn a wake child (EMPTY message; it competes for its
                    // OWN family lock and drains the queue oldest-id-first).
                    Some(fam) => {
                        for lock in fam.into_locks() {
                            lock.release().map_err(|e| Error::Lockfile {
                                key: key.clone(),
                                source: e,
                            })?;
                        }
                        // Lazy: if `delivered` is already ready, this future
                        // is never polled and no child process is launched.
                        let spawn = spawn_child(
                            child.clone(),
                            RichContent::Text(String::new()),
                            seed,
                        );
                        tokio::select! {
                            biased;
                            delivery = &mut delivered => {
                                delivery?;
                                return Ok(Response::Delivered);
                            }
                            res = spawn => match res {
                                // Child won its family and is delivering; loop
                                // into the ACTIVE arm to wait it out.
                                Ok(_) => {}
                                // Child did NOT take the lock. Disambiguate a
                                // lost race from a genuine failure with an
                                // authoritative re-acquire — never by reading
                                // the child's error text.
                                Err(e) => {
                                    match super::locks::try_acquire_family(
                                        ctx.agent_locks(),
                                        ctx.db_client().await?,
                                        &state_dir,
                                        family.clone(),
                                    )
                                    .await?
                                    {
                                        // Someone else holds it → a live agent
                                        // is running → it delivers (or releases
                                        // → re-loop). Not our failure.
                                        None => {}
                                        // Lock free AND ours → no other agent
                                        // runs. DB point-truth settles the
                                        // "child delivered then errored before
                                        // its Id" race.
                                        Some(fam2) => {
                                            for lock in fam2.into_locks() {
                                                lock.release().map_err(|e| Error::Lockfile {
                                                    key: key.clone(),
                                                    source: e,
                                                })?;
                                            }
                                            if crate::db::message_queue::is_active(
                                                &pool, queue_id,
                                            )
                                            .await?
                                            {
                                                return Err(e);
                                            }
                                            return Ok(Response::Delivered);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Resolved delivery route for the non-enqueue flow.
enum Route {
    /// Plain agent ref — nothing to lock or enqueue against; always
    /// spawns a fresh agent carrying the message.
    Ref { child: AgentSelector },
    /// Lockable target: the PRIMARY lock coordinates, the agent's whole
    /// lock `family` (acquired together so none of its tags/labs can be
    /// relocated/detached while live), the queue target (exactly one of
    /// `hierarchy` / `tag` is `Some`), and the selector the spawn child receives.
    Locked {
        dir: std::path::PathBuf,
        key: String,
        family: super::locks::Family,
        hierarchy: Option<String>,
        tag: Option<String>,
        child: AgentSelector,
    },
}

/// Route for a fully resolved `agent_instance_hierarchy`: the AIH
/// lock, queued against the hierarchy, child re-addressed as an
/// explicit Instance (parent + leaf) so it doesn't depend on the
/// child process's own identity.
fn instance_route(state_dir: &std::path::Path, hierarchy: String) -> Route {
    let (dir, key) = super::locks::agent_instance_lock(state_dir, &hierarchy);
    let child = AgentSelector::Instance {
        parent_agent_instance_hierarchy: Some(
            crate::db::tags::parent_of(&hierarchy).to_string(),
        ),
        agent_instance: crate::db::tags::leaf_of(&hierarchy).to_string(),
    };
    Route::Locked {
        dir,
        key,
        family: super::locks::Family::Hierarchy(hierarchy.clone()),
        hierarchy: Some(hierarchy),
        tag: None,
        child,
    }
}

/// Exec a detached `agents spawn` child (stream=true) and return its
/// first item as the unary response. The Ref route passes real
/// `content`; the Locked route passes EMPTY content — `agents spawn`
/// maps that to an empty `messages` array, a wake-up turn whose
/// prompt is the queue drain. The child acquires its OWN family lock
/// at startup (in `agents spawn`) — this process hands it nothing
/// (cross-process lock transfer is unsound on Windows). The child's
/// first item is always its `Id` (chunks are gated behind it); the
/// rest of the stream is dropped and the orphan keeps running.
async fn spawn_child(
    agent: AgentSelector,
    content: RichContent,
    seed: Option<i64>,
) -> Result<Response, Error> {
    let child_request = spawn_sdk::Request {
        path_type: spawn_sdk::Path::AgentsSpawn,
        message: RequestMessage::Inline(content),
        agent,
        dangerous_advanced: Some(spawn_sdk::RequestDangerousAdvanced {
            stream: Some(true),
            seed,
        }),
        base: Default::default(),
    };

    let exe = std::env::current_exe()
        .map_err(|e| Error::Spawn("current_exe".into(), e))?;
    let executor = BinaryExecutor::from_path(exe).detach(true);
    let mut stream = executor
        .execute::<spawn_sdk::Request, spawn_sdk::ResponseItem>(child_request, None)
        .await
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    let first = stream
        .next()
        .await
        .ok_or(Error::EmptyStream)?
        .map_err(|e| Error::Instance(format!("exec agents spawn child: {e}")))?;
    match first {
        spawn_sdk::ResponseItem::Id(agent_instance_hierarchy) => Ok(Response::Id {
            agent_instance_hierarchy,
        }),
        spawn_sdk::ResponseItem::Chunk(_) => Err(Error::Instance(
            "agents spawn child emitted a chunk before its id".to_string(),
        )),
    }
}

pub async fn resolve_message(
    ctx: &Context,
    message: RequestMessage,
) -> Result<RichContent, Error> {
    let (simple, inline, file, python_inline, python_file) = match message {
        RequestMessage::Inline(rich) => return Ok(rich),
        RequestMessage::Simple(s) => (Some(s), None, None, None, None),
        RequestMessage::File(p) => (None, None, Some(p), None, None),
        RequestMessage::PythonInline(code) => (None, None, None, Some(code), None),
        RequestMessage::PythonFile(p) => (None, None, None, None, Some(p)),
    };
    crate::source_resolver::resolve_source(
        ctx,
        simple,
        inline,
        file,
        python_inline,
        python_file,
        RichContent::Text,
    )
    .await
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::request_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::agents::message as sdk;
    use objectiveai_sdk::cli::command::agents::message::response_schema::{Request, Response};

    use crate::context::Context;
    use crate::error::Error;

    pub async fn execute(_ctx: &Context, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
