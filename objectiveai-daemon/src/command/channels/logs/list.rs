//! `channels logs list` — envelope listing (`--all` / `--pending`).
//! Authenticate a channel secret; the role decides which entries
//! `--pending` returns (and advances that role's watermark).

use objectiveai_sdk::cli::command::channels::logs::list::{
    ChannelLogEntry, MessageKind, Request, Response,
};

use crate::context::{GlobalContext, ScopedContext};
use crate::db::channels::{self, Direction, MessageEnvelope};
use crate::error::Error;

/// Map a stored direction to the wire kind (used by `logs open`, whose
/// flat entry still carries a `kind` field).
pub(crate) fn to_kind(direction: Direction) -> MessageKind {
    match direction {
        Direction::Request => MessageKind::Request,
        Direction::Reply => MessageKind::Reply,
        Direction::Publish => MessageKind::Publish,
        Direction::PublishMessage => MessageKind::PublishMessage,
    }
}

/// Resolve the publish item's `message_id` — one extra query, run
/// only when the page actually carries the channel's `publish`
/// envelope (its first entry).
pub(crate) async fn resolve_message_id(
    db: &crate::db::Pool,
    channel_id: &str,
    envelopes: &[MessageEnvelope],
) -> Result<Option<i64>, Error> {
    if envelopes.iter().any(|e| e.direction == Direction::Publish) {
        Ok(channels::publish_message_id(db, channel_id).await?)
    } else {
        Ok(None)
    }
}

/// Map a DB envelope to the wire entry (unix-seconds → RFC3339). The
/// stored full identity is projected to the inline sender-only shape;
/// the entry variant follows the direction: a `publish` seed (the
/// offer) pairs its own id (`details_id`) with the channel's
/// `publish_message` row id (`message_id`, resolved by the caller via
/// [`resolve_message_id`]); a `request` (publisher write) carries the
/// REQUIRED plugin trio — guaranteed present, since `publish` and
/// `logs request` both require a plugin caller — while a `reply`
/// (owner write) carries no plugin identity. The `publish_message`
/// row itself is `None`: it never surfaces as its own entry (the
/// enumeration filters it; this arm is belt-and-braces).
pub(crate) fn to_entry(
    envelope: MessageEnvelope,
    message_id: Option<i64>,
) -> Option<ChannelLogEntry> {
    let identity = envelope.identity;
    let timestamp = crate::db::time::unix_to_rfc3339(envelope.delivered_at);
    // The AIH is always stored (the daemon defaults it in
    // `scope_identity`), so `unwrap_or_default` never actually
    // defaults on a live row.
    let sender = identity.agent_instance_hierarchy.unwrap_or_default();
    match envelope.direction {
        Direction::Publish => Some(ChannelLogEntry::Publish {
            details_id: envelope.id,
            // Present by construction (the seed rows are written in
            // one transaction); 0 only for a corrupted pair.
            message_id: message_id.unwrap_or_default(),
            timestamp,
            sender_agent_instance_hierarchy: sender,
            // Present by construction (`publish` requires a plugin
            // caller).
            plugin_owner: identity.plugin_owner.unwrap_or_default(),
            plugin_name: identity.plugin_name.unwrap_or_default(),
            plugin_version: identity.plugin_version.unwrap_or_default(),
        }),
        Direction::PublishMessage => None,
        Direction::Request => Some(ChannelLogEntry::Request {
            details_id: envelope.id,
            timestamp,
            sender_agent_instance_hierarchy: sender,
            // Present by construction (require_plugin on the write
            // path); `unwrap_or_default` only guards a pre-enforcement
            // legacy row, never a live write.
            plugin_owner: identity.plugin_owner.unwrap_or_default(),
            plugin_name: identity.plugin_name.unwrap_or_default(),
            plugin_version: identity.plugin_version.unwrap_or_default(),
        }),
        Direction::Reply => Some(ChannelLogEntry::Reply {
            details_id: envelope.id,
            timestamp,
            sender_agent_instance_hierarchy: sender,
            // Optional: the replier is usually not a plugin, but a
            // plugin reply passes its trio through verbatim.
            plugin_owner: identity.plugin_owner,
            plugin_name: identity.plugin_name,
            plugin_version: identity.plugin_version,
        }),
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
    let message_id = resolve_message_id(&db, &request.channel_id, &envelopes).await?;
    Ok(Response {
        entries: envelopes
            .into_iter()
            .filter_map(|envelope| to_entry(envelope, message_id))
            .collect(),
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
