//! Error type for the postgres-backed CLI state.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("inline JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
    #[error("cannot reach the database at {url} ({source}); the daemon starts the local objectiveai-db on demand — retry, or point `db config address` at a reachable postgres")]
    DbUnreachable {
        /// Connect URL with the password masked.
        url: String,
        source: Box<sqlx::Error>,
    },
}
