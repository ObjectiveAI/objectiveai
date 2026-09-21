//! What a Python continuation holds, and where it lives.

use diverge_provider_sdk::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use rmcp::model::ContentBlock;
use sqlx::PgPool;

/// A continuation, opened.
///
/// Opaque to everyone but this container: what it holds is the
/// conversation's own history — each turn's user prompt and the
/// chunks the loop produced, in order — as a JSON array. Resuming is
/// reading it back and handing it, whole, to the script: the array
/// IS the `input` the script reads, so the stored form and the fed
/// form are one form.
///
/// # It lives in the caller's database
///
/// One row of one table, reached through the proxy's loopback pgwire:
/// [`load`](Self::load) reads it when the loop starts — no row is a
/// fresh start, a row that will not open as a history is an error, by
/// rule — and [`save`](Self::save) replaces it at every point the
/// history is at rest, so a container that dies loses nothing that
/// was ever whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Continuation(pub Vec<ContinuationItem>);

/// One entry in the history.
///
/// Untagged, and unambiguous without a tag: a chunk serializes as a
/// JSON object — its `type` member inside — and a prompt as a JSON
/// array of MCP content blocks. An object and an array cannot
/// collide.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum ContinuationItem {
    /// One chunk the loop produced.
    Chunk(AgenticLoopChunk),
    /// One turn's user message, as its content blocks.
    Prompt(Vec<ContentBlock>),
}

/// The table: one row, `id` pinned to `1`, the history as jsonb.
const CREATE: &str = "CREATE TABLE IF NOT EXISTS continuation (\
    id smallint PRIMARY KEY CHECK (id = 1), state jsonb NOT NULL)";

/// The row, if there is one.
const SELECT: &str = "SELECT state FROM continuation WHERE id = 1";

/// The row, written or replaced.
const UPSERT: &str = "INSERT INTO continuation (id, state) VALUES (1, $1) \
    ON CONFLICT (id) DO UPDATE SET state = EXCLUDED.state";

impl Continuation {
    /// Read the history the database holds, making the table if this
    /// is the first run to look.
    pub async fn load(pool: &PgPool) -> Result<Option<Self>, Error> {
        sqlx::query(CREATE).execute(pool).await?;
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as(SELECT).fetch_optional(pool).await?;
        match row {
            None => Ok(None),
            Some((state,)) => serde_json::from_value(state)
                .map(|items| Some(Continuation(items)))
                .map_err(Error::Parse),
        }
    }

    /// Replace the history the database holds with this one.
    pub async fn save(&self, pool: &PgPool) -> Result<(), Error> {
        let state = serde_json::to_value(&self.0).map_err(Error::Parse)?;
        sqlx::query(UPSERT).bind(state).execute(pool).await?;
        Ok(())
    }
}

/// A history that could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The database would not answer.
    #[error("the continuation's database failed: {0}")]
    Database(#[from] sqlx::Error),
    /// The row is there and is not a history — or one would not
    /// serialize, which plain data never fails to do.
    #[error("the continuation would not parse: {0}")]
    Parse(serde_json::Error),
}

impl Error {
    /// The failure as JSON, the shape every failure on the stream has.
    pub fn message(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": "continuation",
            "error": self.to_string(),
        })
    }
}
