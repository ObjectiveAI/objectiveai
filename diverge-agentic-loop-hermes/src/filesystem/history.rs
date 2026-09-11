//! A session's transcript, read back for the run to resume from.

use sqlx::{Connection as _, Row as _};

use super::continuation::db::open;
use super::{HERMES_HOME, HistoryError, Message, STATE_DB};

/// Hermes stores structured (list or dict) message content as JSON
/// behind this sentinel; anything else in the column is the text
/// itself (`hermes_state.py`, `_encode_content` / `_decode_content`).
const JSON_PREFIX: &str = "\u{0}json:";

/// The session's transcript, as `/v1/runs` can take it: its `user`
/// and `assistant` rows, in order, as text.
///
/// `/v1/runs` loads no history of its own — `session_id` only names
/// the row a turn records into — and what it accepts as
/// `conversation_history` is `role` and `content` strings, nothing
/// else. Hermes's own loader (`get_messages_as_conversation`)
/// returns richer rows — tool calls, tool results, reasoning — but
/// none of that survives the endpoint, and the loop's pre-request
/// repair drops a `tool` message that matches no preceding call
/// anyway. So this is that loader reduced to what can arrive: the
/// same rows (this session only, `active = 1`, insertion order), the
/// two roles that carry text, the text. A resumed run sees what was
/// said, not what was called; the database holds all of it still.
///
/// Content is stored as plain text, or — for a multimodal user turn
/// — as JSON behind a sentinel: an array of parts, of which the
/// `text` parts are joined with a newline (images have no text form
/// on this endpoint). An empty result (a pure tool-call turn, an
/// image-only turn) is no message, and the row is skipped.
///
/// Read on its own connection while the gateway runs: WAL mode
/// lets a reader in beside the writer.
pub async fn history(session_id: &str) -> Result<Vec<Message>, HistoryError> {
    let db = std::path::Path::new(HERMES_HOME).join(STATE_DB);
    let mut connection = open(&db).await?;
    let rows = sqlx::query(
        "SELECT role, content FROM messages \
         WHERE session_id = ? AND active = 1 \
         AND role IN ('user', 'assistant') \
         ORDER BY id",
    )
    .bind(session_id)
    .persistent(false)
    .fetch_all(&mut connection)
    .await?;
    connection.close().await?;

    let mut messages = Vec::with_capacity(rows.len());
    for row in rows {
        let role = row.get::<String, _>(0);
        let Some(content) = row.get::<Option<String>, _>(1) else {
            continue;
        };
        let content = text(&content)?;
        if content.is_empty() {
            continue;
        }
        messages.push(Message { role, content });
    }
    Ok(messages)
}

/// The text of a stored content column: itself, or the text parts
/// of the JSON it holds behind the sentinel.
fn text(content: &str) -> Result<String, HistoryError> {
    let Some(json) = content.strip_prefix(JSON_PREFIX) else {
        return Ok(content.to_string());
    };
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(HistoryError::Content)?;
    let parts = match value {
        serde_json::Value::Array(parts) => parts,
        serde_json::Value::String(text) => return Ok(text),
        _ => return Ok(String::new()),
    };
    Ok(parts
        .iter()
        .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
        .collect::<Vec<_>>()
        .join("\n"))
}
