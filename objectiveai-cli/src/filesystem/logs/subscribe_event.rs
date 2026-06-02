//! `SubscribeEvent` — NDJSON line type written by cli-stream's
//! outbound per-agent event pipe (`pipes/<agent_instance_hierarchy>/events.sock`)
//! and consumed by `objectiveai-cli agents read subscribe`.
//!
//! This is an implementation detail of the cli-stream / subscribe
//! handshake — not a stable wire type. It lives in the SDK so both
//! crates can deserialize the same shape without pulling in
//! `interprocess`. The transport (AF_UNIX local socket) is owned by
//! the consumers; only the line format is shared here.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::db::schema::MessageKind;

/// One event written to the outbound pipe per NDJSON line.
///
/// - [`SubscribeEvent::Row`] follows every successful insert into
///   `messages` for that agent. `message_kind` is informational —
///   subscribers still drain via the watermark to stay sync-proof.
/// - [`SubscribeEvent::StreamEnd`] is written exactly once, after the
///   final row event for the stream. No more events will arrive on
///   this pipe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[schemars(rename = "filesystem.logs.SubscribeEvent")]
pub enum SubscribeEvent {
    #[schemars(title = "Row")]
    Row { message_kind: MessageKind },
    #[schemars(title = "StreamEnd")]
    StreamEnd,
}
