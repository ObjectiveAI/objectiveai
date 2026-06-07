//! Newtype wrapper around `sqlx::PgPool` so the CLI's
//! [`crate::context::Context`] holds a domain-typed handle.
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
