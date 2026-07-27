//! The plugin's database.
//!
//! Every plugin container gets its own Postgres, tunnelled in by the
//! laboratory host: the host binds a listener, hands the daemon's
//! role, password and database through, and stamps the assembled URL
//! as `OBJECTIVEAI_POSTGRES_URL` at create time. A plugin therefore
//! never configures a database — it asks for one.
//!
//! ```no_run
//! # async fn example() -> Result<(), objectiveai_mcp_plugin_framework::db::Error> {
//! let pool = objectiveai_mcp_plugin_framework::db::connect(Default::default()).await?;
//! # let _ = pool;
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

/// A handle to the plugin's database.
///
/// A POOL rather than a single connection: it is cheap to clone,
/// shares one set of connections between every clone, and is what
/// every `sqlx` query API already takes. Hold one and clone it around.
pub type Pool = sqlx::PgPool;

/// Everything [`connect`] can fail with.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// No `OBJECTIVEAI_POSTGRES_URL` in the environment.
    ///
    /// Not a misconfiguration a plugin can fix: the laboratory host
    /// stamps this variable on every plugin container it creates, so
    /// its absence means this process is not running as one.
    #[error(
        "no database: OBJECTIVEAI_POSTGRES_URL is unset, which means this process is not running as a plugin container"
    )]
    NoDatabase,
    /// The URL was there and the connection still failed — the tunnel
    /// is down, the credentials are stale, or Postgres refused.
    #[error("connect to the plugin database")]
    Connect(#[source] sqlx::Error),
}

/// How the pool is built.
///
/// [`Default`] suits a plugin container as it actually runs: nothing
/// held open when idle, a generous ceiling for when it is not, and a
/// bounded wait. Reach for the fields only to tighten them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Ceiling on open connections.
    ///
    /// High by default, and cheap to leave high: the pool opens
    /// connections ON DEMAND and holds none idle (see
    /// [`Config::min_connections`]), so a plugin that uses three opens
    /// three. The ceiling only bites under real concurrency, and a
    /// plugin that fans out should not have to discover a config knob
    /// to do so.
    ///
    /// It is deliberately NOT a throttle. The daemon's Postgres runs
    /// with `max_connections=1000` shared across every plugin
    /// container alive at once, so this ceiling sits above the
    /// server's — exhaustion surfaces as the server refusing
    /// ("too many clients already"), not as the pool queueing. That is
    /// the right place for it to surface: one plugin should not be
    /// silently rationed to a fraction it might not need.
    pub max_connections: u32,
    /// Connections kept open when idle. Zero by default — a container
    /// that finishes its completion in a second should not have held
    /// connections open waiting for work that will never come.
    pub min_connections: u32,
    /// How long `acquire` waits for a free connection before giving
    /// up. Bounded so a query storm surfaces as an error rather than
    /// as a plugin that hangs until the completion times out.
    pub acquire_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_connections: 1024,
            min_connections: 0,
            acquire_timeout: Duration::from_secs(30),
        }
    }
}

/// Connect to the plugin's database.
///
/// The URL is NOT a parameter: it comes from the environment the
/// laboratory host stamped, which is the only authority on where this
/// container's Postgres lives. A plugin that could pass its own URL
/// could dial somewhere it was never granted.
///
/// Connects EAGERLY, so a bad tunnel or stale credentials surface here
/// — at startup, with a clear error — rather than at the first query
/// somewhere deep in a tool call.
pub async fn connect(config: Config) -> Result<Pool, Error> {
    let url = crate::environment::postgres_url().ok_or(Error::NoDatabase)?;
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(url)
        .await
        .map_err(Error::Connect)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A generous ceiling, nothing held idle, and a bounded wait —
    /// the pool never costs what it does not use, but never throttles
    /// a plugin that genuinely fans out either.
    #[test]
    fn default_config_is_generous_but_bounded() {
        let config = Config::default();
        assert_eq!(config.max_connections, 1024);
        assert_eq!(config.min_connections, 0);
        assert_eq!(config.acquire_timeout, Duration::from_secs(30));
    }

    /// The missing-URL case has to be a named error a caller can match
    /// on, not an opaque connection failure — it means "not running as
    /// a plugin", which is a different problem with a different fix.
    #[tokio::test]
    async fn no_url_is_its_own_error() {
        if crate::environment::postgres_url().is_some() {
            // A real plugin container is not where this test runs, but
            // refuse to assert the opposite of the truth if it does.
            return;
        }
        assert!(matches!(
            connect(Config::default()).await,
            Err(Error::NoDatabase),
        ));
    }
}
