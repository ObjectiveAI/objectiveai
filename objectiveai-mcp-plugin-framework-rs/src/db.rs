//! The plugin's database.
//!
//! Every plugin container gets its own Postgres, tunnelled in by the
//! laboratory host: the host binds a listener, hands the daemon's
//! role, password and database through, and stamps the assembled URL
//! as `OBJECTIVEAI_POSTGRES_URL` at create time. A plugin therefore
//! never configures a database — it asks for one.
//!
//! ```no_run
//! # use std::sync::Arc;
//! # async fn example() -> Result<(), Arc<objectiveai_mcp_plugin_framework::db::Error>> {
//! // Connects once, however many times or from however many tasks
//! // this is called.
//! let pool = objectiveai_mcp_plugin_framework::db::connect(Default::default()).await?;
//! # let _ = pool;
//! # Ok(())
//! # }
//! ```
//!
//! **[`connect`] connects at most once.** A plugin has one database
//! for its whole life, so handing out a second pool would only spend
//! connections on a duplicate of the first. What the memoization has
//! to get right is what happens when callers overlap, and when the
//! connection fails:
//!
//! - **Once it succeeds, that is the answer forever.** Later calls
//!   return the same pool without taking a lock at all.
//! - **Overlapping callers share ONE attempt.** The first starts it;
//!   everyone arriving while it runs waits on that same attempt and
//!   receives its outcome, success or failure alike. Only one
//!   connection attempt is ever in flight.
//! - **Failure is not cached.** Once a failed attempt finishes, the
//!   slot is cleared and the next call starts fresh. A tunnel that was
//!   not up yet does not poison the process.
//!
//! Sharing an outcome means cloning it, so the error is an
//! `Arc<Error>` — `sqlx::Error` is not `Clone`, and every waiter on a
//! failed attempt gets that same error.

use std::sync::{Arc, LazyLock, Mutex, OnceLock};
use std::time::Duration;

use futures::FutureExt as _;
use futures::future::{BoxFuture, Shared};
use sqlx::postgres::PgPoolOptions;

/// A handle to the plugin's database.
///
/// A POOL rather than a single connection: it is cheap to clone,
/// shares one set of connections between every clone, and is what
/// every `sqlx` query API already takes. Hold one and clone it around.
pub type Pool = sqlx::PgPool;

/// Everything [`connect`] can fail with.
///
/// Handed out as `Arc<Error>`, because overlapping callers share one
/// attempt and [`Shared`] gives each of them a CLONE of its output —
/// which `sqlx::Error` is not.
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
///
/// Only the attempt that actually CONNECTS is shaped by this — see
/// [`connect`].
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
    /// It is deliberately NOT a throttle, and it matches the server's
    /// own `max_connections=1024`. Exhaustion therefore surfaces as
    /// Postgres refusing ("too many clients already"), not as this
    /// pool queueing — which is the right place for it: one plugin
    /// should not be silently rationed to a fraction of a budget it
    /// may be the only claimant on.
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

/// The connection, once it exists. TERMINAL: nothing ever replaces or
/// clears it, which is what lets the fast path skip the lock entirely.
static POOL: OnceLock<Pool> = OnceLock::new();

/// The attempt currently in flight, if any.
///
/// A plain `Mutex`, and not something cleverer, because it is only
/// ever held to clone a handle or clear a slot — never across an await
/// — and after the first success [`POOL`] short-circuits before this
/// is even looked at.
static ATTEMPT: LazyLock<Mutex<Attempt>> = LazyLock::new(|| {
    Mutex::new(Attempt {
        current: None,
        generation: 0,
    })
});

struct Attempt {
    /// The live attempt, and the generation identifying it.
    current: Option<(u64, Shared<BoxFuture<'static, Result<Pool, Arc<Error>>>>)>,
    /// Bumped per attempt, so a finishing attempt can tell whether the
    /// slot still holds ITSELF before clearing it. Without it, a slow
    /// failure could clear the slot out from under a newer attempt
    /// that had already replaced it.
    generation: u64,
}

/// Connect to the plugin's database, or return the connection already
/// made.
///
/// The URL is NOT a parameter: it comes from the environment the
/// laboratory host stamped, which is the only authority on where this
/// container's Postgres lives. A plugin that could pass its own URL
/// could dial somewhere it was never granted.
///
/// `config` shapes only the attempt that actually connects. A call
/// that finds a pool already made returns it as-is, and a call that
/// joins a running attempt gets that attempt's config — so passing
/// different configs from different call sites is not meaningful, and
/// whoever connects first decides.
///
/// Connects EAGERLY, so a bad tunnel or stale credentials surface here
/// rather than at the first query somewhere deep in a tool call.
pub async fn connect(config: Config) -> Result<Pool, Arc<Error>> {
    coalesce(|| connect_once(config).boxed()).await
}

/// The memoization itself, separated from what it memoizes.
///
/// `start` is only called when there is no pool and no attempt in
/// flight. Splitting it out is what makes the coalescing testable: the
/// real attempt fails synchronously when there is no database, so it
/// never suspends, so nothing can ever overlap it — and a test that
/// drove the real thing would prove nothing about joining.
async fn coalesce(
    start: impl FnOnce() -> BoxFuture<'static, Result<Pool, Arc<Error>>>,
) -> Result<Pool, Arc<Error>> {
    // After the first success, this is the entire function.
    if let Some(pool) = POOL.get() {
        return Ok(pool.clone());
    }

    // Join the running attempt, or start one. The guard is dropped
    // before the await: a lock held across a connection attempt would
    // serialize the very callers this exists to coalesce.
    let (generation, attempt) = {
        let mut state = ATTEMPT.lock().unwrap_or_else(|e| e.into_inner());

        // RETIRE a finished attempt before deciding anything.
        //
        // Without this there is a window — between an attempt
        // resolving and a waiter clearing the slot — in which the slot
        // still holds a FINISHED attempt. A caller arriving then would
        // join it and receive its cached outcome, which for a failure
        // is exactly the stale error that is supposed to be
        // unreachable. Worse, if every waiter is dropped in that
        // window, nobody ever clears and the next caller is guaranteed
        // the dead attempt.
        //
        // Deciding it HERE, under the same lock that hands out joins,
        // is what makes the boundary exact: "arrived before it
        // finished" and "arrived after" are separated by this critical
        // section rather than by a clear that races it.
        let finished = state.current.as_ref().and_then(|(_, a)| a.peek().cloned());
        match finished {
            // It succeeded and we are the first to notice. Publish and
            // hand it back: starting a fresh connection here would
            // waste one, and joining a completed future would leave it
            // in the slot forever.
            Some(Ok(pool)) => {
                state.current = None;
                drop(state);
                let _ = POOL.set(pool.clone());
                return Ok(pool);
            }
            // It failed, and we arrived after. Retire it and start over
            // — the whole point of not caching failure.
            Some(Err(_)) => state.current = None,
            // Still running, so joinable.
            None => {}
        }

        match &state.current {
            Some((generation, attempt)) => (*generation, attempt.clone()),
            None => {
                state.generation += 1;
                let generation = state.generation;
                let attempt = start().shared();
                state.current = Some((generation, attempt.clone()));
                (generation, attempt)
            }
        }
    };

    match attempt.await {
        Ok(pool) => {
            // Whichever waiter arrives first wins the `set`; the rest
            // return the same pool regardless, since they are all
            // holding clones of one `PgPool`.
            let _ = POOL.set(pool.clone());
            // Release the slot too. `POOL` is the answer from here on,
            // so holding the completed attempt would retain a finished
            // future and a duplicate pool handle for the life of the
            // process.
            let mut state = ATTEMPT.lock().unwrap_or_else(|e| e.into_inner());
            if state.current.as_ref().is_some_and(|(g, _)| *g == generation) {
                state.current = None;
            }
            Ok(pool)
        }
        Err(error) => {
            // Clear the slot so the NEXT caller starts fresh — but
            // only if it still holds THIS attempt, since a newer one
            // may already have replaced it.
            let mut state = ATTEMPT.lock().unwrap_or_else(|e| e.into_inner());
            if state.current.as_ref().is_some_and(|(g, _)| *g == generation) {
                state.current = None;
            }
            Err(error)
        }
    }
}

/// One connection attempt, with no memoization of its own.
async fn connect_once(config: Config) -> Result<Pool, Arc<Error>> {
    let url = crate::environment::postgres_url()
        .ok_or_else(|| Arc::new(Error::NoDatabase))?;
    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(config.acquire_timeout)
        .connect(url)
        .await
        .map_err(|e| Arc::new(Error::Connect(e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test here depends on there being no database to reach.
    /// Inside a real plugin container there is one, and asserting the
    /// opposite of the truth would be worse than skipping.
    fn outside_a_plugin_container() -> bool {
        crate::environment::postgres_url().is_none()
    }

    /// The memoization is PROCESS-global, so tests that count attempts
    /// would otherwise count each other — libtest runs them on
    /// separate threads by default. Every test touching `ATTEMPT`
    /// takes this first.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn exclusive() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// A generous ceiling, nothing held idle, and a bounded wait — the
    /// pool never costs what it does not use, but never throttles a
    /// plugin that genuinely fans out either.
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
        let _serial = exclusive();
        if !outside_a_plugin_container() {
            return;
        }
        let error = connect(Config::default()).await.expect_err("no database");
        assert!(matches!(*error, Error::NoDatabase));
    }

    /// Failure is not cached: each later call starts over. A plugin
    /// that raced the tunnel coming up must be able to recover, which
    /// a remembered error would prevent forever.
    #[tokio::test]
    async fn a_failed_attempt_leaves_no_trace() {
        let _serial = exclusive();
        if !outside_a_plugin_container() {
            return;
        }
        let before = ATTEMPT.lock().unwrap().generation;
        for _ in 0..3 {
            assert!(connect(Config::default()).await.is_err());
        }
        let state = ATTEMPT.lock().unwrap();
        assert!(
            state.current.is_none(),
            "the slot is empty, so the next caller starts fresh",
        );
        assert_eq!(
            state.generation - before,
            3,
            "and each sequential call really did start its own attempt",
        );
    }

    /// Overlapping callers share ONE attempt and all receive its
    /// outcome — the coalescing that stops a burst of tool calls
    /// opening a burst of connections.
    ///
    /// Driven through [`coalesce`] with an attempt that SUSPENDS,
    /// because the real one does not: with no database it fails
    /// without ever yielding, so callers can never overlap it and a
    /// test against it would pass while proving nothing.
    #[tokio::test]
    async fn overlapping_callers_share_one_attempt() {
        let _serial = exclusive();
        let before = ATTEMPT.lock().unwrap().generation;

        let results = futures::future::join_all((0..8).map(|_| {
            coalesce(|| {
                async {
                    // A real pause, not a single `yield_now`: one
                    // reschedule guarantees only ONE further poll,
                    // which can let the attempt finish before the last
                    // callers have been polled even once. A timer holds
                    // the window open for all of them.
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Err(Arc::new(Error::NoDatabase))
                }
                .boxed()
            })
        }))
        .await;

        let after = ATTEMPT.lock().unwrap().generation;
        assert_eq!(results.len(), 8);
        assert!(results.iter().all(|r| r.is_err()), "all eight see the failure");
        assert_eq!(
            after - before,
            1,
            "eight concurrent callers, one attempt between them",
        );
    }

    /// A caller arriving AFTER a failed attempt resolved must start
    /// over, not inherit its error.
    ///
    /// The natural window is too narrow to hit on purpose, so the
    /// state it produces is built directly: a slot holding an attempt
    /// that has already finished with an error, exactly as it looks
    /// between resolution and the waiter clearing it.
    #[tokio::test]
    async fn a_finished_failure_is_retired_not_joined() {
        let _serial = exclusive();

        // An attempt that is already resolved, with a DISTINGUISHABLE
        // error.
        let stale: Shared<BoxFuture<'static, Result<Pool, Arc<Error>>>> =
            async { Err(Arc::new(Error::NoDatabase)) }.boxed().shared();
        assert!(
            stale.clone().now_or_never().is_some(),
            "resolved, so it is what the slot holds after a failure",
        );

        let before = {
            let mut state = ATTEMPT.lock().unwrap();
            state.generation += 1;
            state.current = Some((state.generation, stale));
            state.generation
        };

        // A fresh attempt with a different error, so the two are
        // telling apart.
        let outcome = coalesce(|| {
            async { Err(Arc::new(Error::Connect(sqlx::Error::PoolClosed))) }.boxed()
        })
        .await;

        let error = outcome.expect_err("still no database");
        assert!(
            matches!(*error, Error::Connect(_)),
            "the FRESH attempt answered, not the retired one",
        );
        assert_eq!(
            ATTEMPT.lock().unwrap().generation - before,
            1,
            "and it really was a new attempt",
        );
        assert!(
            ATTEMPT.lock().unwrap().current.is_none(),
            "which then retired itself in turn",
        );
    }

    /// And once that shared attempt has failed, the slot is empty
    /// again — the eight callers coalesced, but they did not leave the
    /// failure behind for a ninth.
    #[tokio::test]
    async fn a_shared_failure_still_clears_the_slot() {
        let _serial = exclusive();
        let _ = coalesce(|| {
            async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Err(Arc::new(Error::NoDatabase))
            }
            .boxed()
        })
        .await;
        assert!(ATTEMPT.lock().unwrap().current.is_none());
    }
}
