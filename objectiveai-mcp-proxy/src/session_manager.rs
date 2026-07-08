//! Session registry.
//!
//! Maps the **objectiveai response id** (`X-OBJECTIVEAI-RESPONSE-ID`) to
//! the [`Session`] holding that response's live upstream MCP
//! connections. Routing is keyed entirely by response id; the
//! `Mcp-Session-Id` the proxy mints on `initialize` is a plain random
//! UUID returned for MCP-spec compliance only (so 3rd-party clients like
//! the Claude Agent SDK get a valid session header) — the proxy never
//! routes on it.
//!
//! All per-session dispatch (list, call, read) lives on [`Session`]
//! itself; this file only packs the opened connections into a [`Session`]
//! under their response id and looks sessions back up.
//!
//! **The pending-initialization gate.** `initialize`'s fresh-connect
//! path dials every upstream before registering the session, and client
//! requests (`tools/list`, `tools/call`, `resources/*`, `servers/list`)
//! can arrive INSIDE that window — notably from an upstream server that
//! calls back into the proxy while it is itself being connected. Rather
//! than 404ing, those requests use [`SessionManager::get_or_wait`]: a
//! miss on a response id whose initial connect is in flight parks until
//! the initializer finishes (success or failure), then resolves against
//! the final state. Ids that are neither registered nor initializing
//! still miss immediately.

use std::sync::Arc;

use dashmap::DashMap;
use indexmap::IndexMap;

use crate::reverse_channel::Upstream;
use crate::session::Session;

/// Maps an objectiveai response id to its [`Session`] state.
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: DashMap<String, Arc<Session>>,
    /// Response ids whose initial connect is in flight. The receiver
    /// resolves (sender dropped) when the initializer finishes —
    /// success or failure alike; the entry is removed by the
    /// [`InitGuard`]'s `Drop`, so it can never outlive its
    /// initializer.
    pending: Arc<DashMap<String, tokio::sync::watch::Receiver<()>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register the upstreams opened for one objectiveai response under
    /// its response id. Builds the `prefix -> Upstream` routing map (see
    /// [`build_prefix_map`]) and inserts a fresh [`Session`]. An existing
    /// entry for the same response id is replaced.
    pub fn add(&self, response_id: String, connections: Vec<Upstream>) {
        let by_prefix = build_prefix_map(connections);
        self.sessions
            .insert(response_id, Arc::new(Session::new(by_prefix)));
    }

    /// Cheap clone-out of a [`Session`] by response id — never holds a
    /// DashMap guard across the await boundary.
    pub fn get(&self, response_id: &str) -> Option<Arc<Session>> {
        self.sessions.get(response_id).map(|e| e.value().clone())
    }

    /// Mark `response_id`'s initial connect as in flight. Client-request
    /// lookups via [`Self::get_or_wait`] park until the returned
    /// [`InitGuard`] drops — which the initializer does on EVERY exit
    /// path (registered, connect failure, ban teardown, panic), since
    /// release lives in `Drop`.
    pub fn begin_initializing(&self, response_id: &str) -> InitGuard {
        let (tx, rx) = tokio::sync::watch::channel(());
        self.pending.insert(response_id.to_string(), rx);
        InitGuard {
            pending: Arc::clone(&self.pending),
            response_id: response_id.to_string(),
            _tx: tx,
        }
    }

    /// The in-flight marker for `response_id`, if its initial connect is
    /// currently running. Clone-out — no guard held across awaits (same
    /// discipline as [`Self::get`]).
    pub fn initializing(
        &self,
        response_id: &str,
    ) -> Option<tokio::sync::watch::Receiver<()>> {
        self.pending.get(response_id).map(|e| e.value().clone())
    }

    /// [`Self::get`], but a miss on a response id whose initial connect
    /// is in flight WAITS for the initializer to finish and resolves
    /// against the final state: `Some` when the connect registered the
    /// session, `None` when it failed (or the id was never
    /// initializing). Boundedness is inherited from the initializer —
    /// the guard drops when `initialize` returns, which is itself
    /// bounded by the per-upstream connect/probe timeouts.
    pub async fn get_or_wait(&self, response_id: &str) -> Option<Arc<Session>> {
        loop {
            if let Some(session) = self.get(response_id) {
                return Some(session);
            }
            let Some(mut rx) = self.initializing(response_id) else {
                // Close the get→pending gap: an initializer may have
                // registered + released between our two probes.
                return self.get(response_id);
            };
            // Sender dropped (Err) ⇒ the initializer finished; drain
            // any intermediate change notifications until then.
            while rx.changed().await.is_ok() {}
            // Re-probe: session present on success. On failure the
            // pending entry is gone too, so the next miss exits via
            // the else branch above.
        }
    }

    /// Remove a session from the registry by response id. Returns
    /// `Some(_)` if a session was present, `None` if the id was unknown.
    ///
    /// Once every `Arc<Session>` to the removed session has dropped, the
    /// session's `IndexMap<String, Connection>` drops, every `Connection`'s
    /// `Drop` fires its upstream's wakeup signal, and each upstream's
    /// listener task wakes to re-check liveness. The listener sees
    /// `Arc::strong_count == 1` (only itself) and exits, which drops the
    /// inner state and closes the upstream HTTP session.
    pub fn remove(&self, response_id: &str) -> Option<Arc<Session>> {
        self.sessions.remove(response_id).map(|(_, session)| session)
    }
}

/// The initializer's hold on a response id's pending-initialization
/// marker — see [`SessionManager::begin_initializing`]. Dropping it
/// removes the marker and wakes every [`SessionManager::get_or_wait`]
/// parked on the id.
pub struct InitGuard {
    pending: Arc<DashMap<String, tokio::sync::watch::Receiver<()>>>,
    response_id: String,
    /// Held only so its drop closes the watch channel.
    _tx: tokio::sync::watch::Sender<()>,
}

impl Drop for InitGuard {
    fn drop(&mut self) {
        // Remove-then-close: a waiter that re-probes after waking must
        // not find the stale marker.
        self.pending.remove(&self.response_id);
    }
}

/// Parse an `MCP_ENCRYPTION_KEY` env-var value: a single base64-encoded
/// 32-byte key. Empty string → `None`. Malformed → `Err`.
///
/// Retained as a no-op compatibility shim: the proxy no longer encrypts
/// session ids (they're plain UUIDs now), but the API crate still parses
/// and forwards the configured key. Kept so that the env plumbing in the
/// API/CLI compiles unchanged; the parsed value is ignored by the proxy.
/// Removal is deferred to a later step of the session-id refactor.
pub fn parse_key_env(s: &str) -> Result<Option<[u8; 32]>, String> {
    use base64::Engine;
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(trimmed)
        .map_err(|e| format!("MCP_ENCRYPTION_KEY: not valid base64: {e}"))?;
    let key: [u8; 32] = decoded.try_into().map_err(|got: Vec<u8>| {
        format!(
            "MCP_ENCRYPTION_KEY: expected 32 bytes after base64-decode, got {}",
            got.len(),
        )
    })?;
    Ok(Some(key))
}

/// Normalize a server name or version into a routing-prefix-safe token:
/// every `_` and `.` becomes `-`. The resulting prefix is free of both the
/// split separator (`_`) and `.`, so the first `_` in a prefixed identifier
/// is always the prefix/original-name boundary.
fn normalize_prefix_token(s: &str) -> String {
    s.replace(['_', '.'], "-")
}

/// Build the `prefix -> Connection` routing map.
///
/// Connections are sorted by `url` first so the prefixes and indices are
/// stable across calls — a client that re-lists must see the exact tool
/// names it already holds.
///
/// Each connection's prefix escalates only as far as needed for global
/// uniqueness:
///   1. `normalize(server_info.name)`
///   2. on collision -> `{name}-{normalize(version)}` (all colliding members)
///   3. still colliding -> `{name}-{version}-{index}` (index = url-sorted
///      position, globally unique, so this tier always resolves)
/// Uniqueness is re-checked over the full set after each tier so a rare
/// cross-tier collision escalates too.
fn build_prefix_map(
    mut connections: Vec<Upstream>,
) -> IndexMap<String, Upstream> {
    connections.sort_by(|a, b| a.url().cmp(b.url()));
    let n = connections.len();

    let names: Vec<String> = connections
        .iter()
        .map(|c| normalize_prefix_token(c.server_name()))
        .collect();
    let versions: Vec<String> = connections
        .iter()
        .map(|c| normalize_prefix_token(c.server_version()))
        .collect();

    // tier: 1 = name, 2 = name-version, 3 = name-version-index.
    let prefix_at = |i: usize, tier: u8| -> String {
        match tier {
            1 => names[i].clone(),
            2 => format!("{}-{}", names[i], versions[i]),
            _ => format!("{}-{}-{}", names[i], versions[i], i),
        }
    };

    let mut tier: Vec<u8> = vec![1; n];
    loop {
        let current: Vec<String> = (0..n).map(|i| prefix_at(i, tier[i])).collect();
        let mut counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for p in &current {
            *counts.entry(p.as_str()).or_insert(0) += 1;
        }
        let mut changed = false;
        for i in 0..n {
            if counts[current[i].as_str()] > 1 && tier[i] < 3 {
                tier[i] += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut by_prefix: IndexMap<String, Upstream> = IndexMap::with_capacity(n);
    for (i, connection) in connections.into_iter().enumerate() {
        let key = prefix_at(i, tier[i]);
        // The index tier guarantees uniqueness; a residual duplicate would
        // be a logic bug rather than a real-world collision.
        debug_assert!(
            !by_prefix.contains_key(&key),
            "duplicate routing prefix after escalation: {key}",
        );
        by_prefix.insert(key, connection);
    }
    by_prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_or_wait_immediate_hit_and_miss() {
        let manager = SessionManager::new();
        manager.add("known".to_string(), Vec::new());
        assert!(manager.get_or_wait("known").await.is_some());
        // Neither registered nor initializing: misses without waiting.
        assert!(manager.get_or_wait("unknown").await.is_none());
    }

    #[tokio::test]
    async fn get_or_wait_parks_until_the_initializer_registers() {
        let manager = Arc::new(SessionManager::new());
        let guard = manager.begin_initializing("r1");

        let waiter = {
            let manager = Arc::clone(&manager);
            tokio::spawn(async move { manager.get_or_wait("r1").await })
        };
        // Let the waiter park on the pending marker.
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        manager.add("r1".to_string(), Vec::new());
        drop(guard);
        let session = waiter.await.expect("waiter task");
        assert!(session.is_some());
    }

    #[tokio::test]
    async fn get_or_wait_resolves_none_when_the_connect_fails() {
        let manager = Arc::new(SessionManager::new());
        let guard = manager.begin_initializing("r1");

        let waiter = {
            let manager = Arc::clone(&manager);
            tokio::spawn(async move { manager.get_or_wait("r1").await })
        };
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());

        // The initializer bails without registering (connect failure /
        // early return): the guard's Drop releases the waiters.
        drop(guard);
        assert!(waiter.await.expect("waiter task").is_none());
    }

    #[tokio::test]
    async fn initializing_marker_lives_exactly_as_long_as_the_guard() {
        let manager = SessionManager::new();
        assert!(manager.initializing("r1").is_none());
        let guard = manager.begin_initializing("r1");
        assert!(manager.initializing("r1").is_some());
        drop(guard);
        assert!(manager.initializing("r1").is_none());
    }

    #[test]
    fn parse_key_env_round_trip() {
        use base64::Engine;
        let key = [0xAAu8; 32];
        let env = base64::engine::general_purpose::STANDARD.encode(key);
        let parsed = parse_key_env(&env).expect("parse").expect("Some");
        assert_eq!(parsed, key);

        assert!(parse_key_env("").unwrap().is_none());
        assert!(parse_key_env("   ").unwrap().is_none());
        assert!(parse_key_env("not-base64!@#").is_err());
        // Wrong-length payload (16 bytes after b64 decode):
        let short =
            base64::engine::general_purpose::STANDARD.encode(&[0u8; 16][..]);
        assert!(parse_key_env(&short).is_err());
    }
}
