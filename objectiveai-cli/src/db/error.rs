//! Error type for the postgres-backed CLI state.
//!
//! `Migrate` exists for forward compatibility but isn't wired to a
//! `sqlx::migrate::MigrateError` `From` impl yet — the `migrate`
//! feature of sqlx pulls `sqlx-sqlite` into the resolution graph (via
//! the `sqlite?/migrate` cross-activation) which conflicts with
//! rustpython's libsqlite3-sys 0.28 pin. We run migrations manually
//! from `init` instead.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migrate: {0}")]
    Migrate(String),
    #[error("inline JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid data: {0}")]
    InvalidData(String),
}
