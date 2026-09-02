//! What `POST /v1/runs` takes.

use serde::Serialize;

use super::Message;

/// The body of `POST /v1/runs`, the fields this container uses of
/// the ones the gateway reads (`api_server.py`, `_handle_runs`).
///
/// Deliberately absent: `provider` (the run's provider is
/// `config.yaml`'s, written before the gateway started) and
/// `previous_response_id` (resumption here is the session's — the
/// tip [`prepare`](crate::filesystem::prepare) named — never the
/// response store's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Request {
    /// This turn's user message, as text.
    pub input: String,
    /// The ephemeral system prompt, layered over Hermes's own for
    /// this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// The session the run records into — the lineage's tip when
    /// resuming, or absent for a fresh session the gateway names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// The conversation so far, as the gateway takes it: role and
    /// content, strings only. `/v1/runs` loads no history on its
    /// own, so a resumed run supplies it here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_history: Option<Vec<Message>>,
    /// The model, in the gateway's naming; absent for the gateway's
    /// default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Per-request model options, passed through as the gateway
    /// reads them — `reasoning: {enabled, effort}` among them, with
    /// the same eight efforts the SDK's ladder has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_options: Option<serde_json::Map<String, serde_json::Value>>,
}
