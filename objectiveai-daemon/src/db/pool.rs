//! Newtype wrapper around `sqlx::PgPool` so the CLI's
//! [`crate::context::GlobalContext`] holds a domain-typed handle.
//!
//! Construction is private to this crate via [`super::init::init`]; the
//! pool itself is concurrency-safe so callers can hold `&Pool`
//! everywhere a sqlite `Arc<Mutex<Connection>>` used to be threaded.

use sqlx::PgPool;

#[derive(Debug, Clone)]
pub struct Pool(pub(crate) PgPool);

impl std::ops::Deref for Pool {
    type Target = PgPool;
    fn deref(&self) -> &PgPool {
        &self.0
    }
}

/// The live pool PLUS the admin coordinates it was built from —
/// what [`super::compartment`] needs to mint derived per-plugin
/// connection strings. Cached as a unit by `Context::db_client`;
/// the password stays in memory exactly as long as the pool it
/// authenticated.
#[derive(Debug, Clone)]
pub struct DbHandle {
    pub pool: Pool,
    /// `host:port` of the server (no scheme, no credentials).
    pub address: String,
    pub admin_user: String,
    pub admin_password: String,
    /// Application database name.
    pub database: String,
}
