//! `user request` — create a pending user request on the `/user` hub,
//! broadcast it to every connected user stream, and BLOCK until the
//! first ACCEPTED reply.
//!
//! The wait is uncapped here by design: the base `--timeout` is
//! enforced generically by the executor's `TimeoutStream`, which
//! DROPS this future on expiry — the [`AbandonGuard`] then ends the
//! request (dropping it from the hub and notifying exactly the
//! connections that saw it with `TimedOut`). The same guard covers a
//! caller disconnect and any early error, so "notify on timeout"
//! needs no timer of its own. Zero connected streams still waits:
//! the request stays pending and replays to later connections.

use objectiveai_sdk::cli::command::AgentArguments;
use objectiveai_sdk::cli::command::user::request::{Request, Response};
use objectiveai_sdk::cli::user_listener::UserRequest;

use crate::context::{GlobalContext, ScopedContext};
use crate::error::Error;

/// Ends the pending request on drop unless disarmed — armed across
/// the whole settle wait.
struct AbandonGuard {
    hub: crate::http::user_routes::UserHub,
    id: String,
    armed: bool,
}

impl AbandonGuard {
    fn disarm(mut self) {
        self.armed = false;
    }
}

impl Drop for AbandonGuard {
    fn drop(&mut self) {
        if self.armed {
            self.hub.abandon(&self.id);
        }
    }
}

pub async fn execute(global: &GlobalContext, scoped: &ScopedContext, request: Request) -> Result<Response, Error> {
    let hubs = global.resident_hubs().ok_or_else(|| {
        Error::Instance("user request requires the resident daemon".to_string())
    })?;
    let id = uuid::Uuid::new_v4().to_string();
    // The payload is DAEMON-authored from the caller's scope — the
    // plugin trio included (unspoofable; only `plugins run` stamps
    // it), both split out per the wire contract and inside the
    // identity bag.
    let payload = UserRequest {
        id: id.clone(),
        plugin_owner: scoped.plugin_owner().map(String::from),
        plugin_repository: scoped.plugin_repository().map(String::from),
        plugin_version: scoped.plugin_version().map(String::from),
        agent_arguments: AgentArguments {
            agent_instance_hierarchy: Some(scoped.agent_instance_hierarchy().to_string()),
            agent_id: scoped.agent_id().map(String::from),
            agent_full_id: scoped.agent_full_id().map(String::from),
            agent_remote: scoped.agent_remote().map(String::from),
            response_id: scoped.response_id().map(String::from),
            response_ids: scoped.response_ids().map(String::from),
            plugin_owner: scoped.plugin_owner().map(String::from),
            plugin_repository: scoped.plugin_repository().map(String::from),
            plugin_version: scoped.plugin_version().map(String::from),
        },
        key: request.key,
        details: request.details,
    };
    let rx = hubs.user.create(&payload);
    let guard = AbandonGuard {
        hub: hubs.user.clone(),
        id,
        armed: true,
    };
    let (identity, reply) = rx.await.map_err(|_| {
        // Unreachable in practice: only this guard ends the request.
        Error::Instance("user request settle channel closed".to_string())
    })?;
    guard.disarm();
    Ok(Response { identity, reply })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::user::request as sdk;
    use objectiveai_sdk::cli::command::user::request::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::user::request as sdk;
    use objectiveai_sdk::cli::command::user::request::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(_global: &GlobalContext, _scoped: &ScopedContext, _request: Request) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
