//! `channels logs list` — envelope listing (`--all` / `--pending`).
//! Authenticate a channel secret; the role decides which entries
//! `--pending` returns (and advances that role's watermark).

use objectiveai_sdk::cli::command::channels::logs::list::{
    ChannelLogEntry, MessageKind, Request, Response,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::channels::{self, Direction, MessageEnvelope};
use crate::error::Error;

/// Map a stored direction to the wire kind.
pub(crate) fn to_kind(direction: Direction) -> MessageKind {
    match direction {
        Direction::Request => MessageKind::Request,
        Direction::Reply => MessageKind::Reply,
    }
}

/// Map a DB envelope to the wire entry (unix-seconds → RFC3339). The
/// stored full identity is projected to the inline sender-only shape:
/// the sender AIH + the originating plugin; the rest of the argument
/// bag stays in the DB, unshown.
pub(crate) fn to_entry(envelope: MessageEnvelope) -> ChannelLogEntry {
    let identity = envelope.identity;
    ChannelLogEntry {
        id: envelope.id,
        timestamp: crate::db::time::unix_to_rfc3339(envelope.delivered_at),
        kind: to_kind(envelope.direction),
        sender_agent_instance_hierarchy: identity.agent_instance_hierarchy,
        plugin_owner: identity.plugin_owner,
        plugin_repository: identity.plugin_repository,
        plugin_version: identity.plugin_version,
    }
}

pub async fn execute(
    global: &GlobalContext,
    _scoped: &ScopedContext,
    request: Request,
) -> Result<Response, Error> {
    let db = global.db_client().await?;
    let auth = channels::channel_auth(&db, &request.channel_id)
        .await?
        .ok_or_else(|| Error::Instance("no such channel".to_string()))?;
    let role = auth.role_of(&request.secret).ok_or_else(|| {
        Error::Instance("secret does not authorize this channel".to_string())
    })?;
    let envelopes = if request.pending {
        channels::read_pending(&db, &request.channel_id, role, request.after_id, request.limit)
            .await?
    } else {
        channels::read_all(&db, &request.channel_id, request.after_id, request.limit).await?
    };
    Ok(Response {
        entries: envelopes.into_iter().map(to_entry).collect(),
    })
}

pub mod request_schema {
    use objectiveai_sdk::cli::command::channels::logs::list as sdk;
    use objectiveai_sdk::cli::command::channels::logs::list::request_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Request)))
    }
}

pub mod response_schema {
    use objectiveai_sdk::cli::command::channels::logs::list as sdk;
    use objectiveai_sdk::cli::command::channels::logs::list::response_schema::{Request, Response};

    use crate::context::{GlobalContext, ScopedContext};
    use crate::error::Error;

    pub async fn execute(
        _global: &GlobalContext,
        _scoped: &ScopedContext,
        _request: Request,
    ) -> Result<Response, Error> {
        Ok(objectiveai_sdk::cli::command::ResponseSchema(schemars::schema_for!(sdk::Response)))
    }
}
