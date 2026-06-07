//! Error type for the postgres-backed CLI state.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("inline JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
}
