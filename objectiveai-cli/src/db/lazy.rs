//! Lazily-initialized pool handle.
//!
//! `Context::new` must NOT connect to postgres eagerly: commands like
//! `config db ...` and `db spawn` have to work before any database
//! exists (they're how you bring one up in the first place). The
//! context therefore holds a [`LazyPool`]; the first `get()` runs
//! [`super::init`] (connect + ensure database + apply schema) and
//! every later call returns the cached pool. Commands that never
//! touch the db never connect.

use std::sync::Arc;

use crate::filesystem::config::DbConfig;

use super::{Error, Pool};

#[derive(Clone)]
pub struct LazyPool {
    config: Arc<DbConfig>,
    cell: Arc<tokio::sync::OnceCell<Pool>>,
}

impl LazyPool {
    pub fn new(config: DbConfig) -> Self {
        Self {
            config: Arc::new(config),
            cell: Arc::new(tokio::sync::OnceCell::new()),
        }
    }

    /// Connect-on-first-use. Concurrent callers coalesce on the same
    /// initialization (tokio `OnceCell` semantics); a failed init is
    /// not cached, so the next call retries.
    pub async fn get(&self) -> Result<&Pool, Error> {
        self.cell
            .get_or_try_init(|| super::init(&self.config))
            .await
    }
}
