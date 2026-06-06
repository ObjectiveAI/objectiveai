//! Parsing of the proxy's two custom session-init headers and fan-out
//! connect over the resulting upstream specs.

use axum::http::HeaderMap;
use futures::future::try_join_all;
use indexmap::IndexMap;
use objectiveai_sdk::mcp::{Client, Connection};

const SERVERS_HEADER: &str = "X-MCP-Servers";
const HEADERS_HEADER: &str = "X-MCP-Headers";
/// Per-request header: when present on a `tools/list` or
/// `resources/list` POST, restricts the fan-out to the single upstream
/// whose URL matches verbatim. Absent → fan out to every upstream
/// (existing behavior). Applies to both list operations so a caller
/// that only knows about one upstream gets a single, focused view.
pub const LIST_FILTER_HEADER: &str = "X-List-Filter";

/// One upstream MCP server the proxy should connect to for a session.
#[derive(Debug)]
struct UpstreamSpec {
    url: String,
    /// Full per-upstream HTTP header map — `Authorization` (when
    /// present), custom `X-*`, etc. The `Mcp-Session-Id` header is
    /// inserted by the connect-time logic and is not part of the
    /// caller-supplied set.
    headers: IndexMap<String, String>,
}

/// Why parsing the two custom session-init headers failed, or why an
/// upstream connect failed.
#[derive(Debug, thiserror::Error)]
pub enum BadInit {
    #[error("{header} is not valid UTF-8")]
    NotUtf8 { header: &'static str },
    #[error("{header} is not valid JSON: {source}")]
    NotJson {
        header: &'static str,
        #[source]
        source: serde_json::Error,
    },
    #[error("upstream connect failed for {url}: {source}")]
    UpstreamConnectFailed {
        url: String,
        #[source]
        source: objectiveai_sdk::mcp::Error,
    },
}

/// HTTP header name used to carry the upstream MCP session id. Stored
/// alongside `Authorization` and any custom headers in the per-upstream
/// header map encoded into the proxy session id.
pub const MCP_SESSION_ID_KEY: &str = "Mcp-Session-Id";

/// Parse the two custom session-init headers and fresh-connect to
/// every upstream URL they describe in parallel.
///
/// This is the no-prior-session path: every URL is connected from
/// scratch, no resume sid. The resume / re-encode flow lives in
/// `mcp::handle_initialize` and uses [`reconnect_from_payload`].
///
/// Headers (all optional):
/// - `X-MCP-Servers`: JSON array of upstream URLs. Empty / absent →
///   empty Vec is returned (the session still initializes, the client
///   just gets nothing from `tools/list` etc).
/// - `X-MCP-Headers`: JSON `{url: {header: value, ...}, ...}`. Per-URL
///   header map applied when the proxy talks to that upstream —
///   `Authorization`, custom `X-*`, etc. URLs not present in the map
///   get an empty header set. There is no separate
///   `X-MCP-Authorization` header; `Authorization` rides as a regular
///   header inside the per-URL map.
///
/// Returns each opened `Connection` paired with the canonical full
/// header set (the headers the proxy used to talk to that upstream,
/// which is what gets encoded into the new session id). The header
/// set is `spec.headers` ∪ `Mcp-Session-Id` (the freshly-minted
/// upstream sid).
///
/// Duplicate URLs in `X-MCP-Servers` are ignored (first-occurrence wins).
/// If any upstream fails to connect, the first such failure is returned
/// as `BadInit::UpstreamConnectFailed` and the remaining in-flight
/// attempts are dropped.
pub async fn connect_all_fresh(
    client: &Client,
    http_headers: &HeaderMap,
) -> Result<Vec<(Connection, IndexMap<String, String>)>, BadInit> {
    let specs = parse_init_headers(http_headers)?;

    // Extract the session-global transient header set from the inbound
    // HeaderMap so we can stamp it on the initial upstream connect.
    // `Session::apply_transient_headers` only fires AFTER
    // `connect_all_fresh` returns; without this pre-stamp, the upstream
    // (e.g. the API's loopback `/objectiveai` route, which requires
    // `X-OBJECTIVEAI-RESPONSE-ID`) rejects the very first connect with
    // 400. Because the mcp client treats every error as transient and
    // loops until `backoff_max_elapsed_time`, the upstream's fast 400
    // turns into a 30-40s spin. Meanwhile the inbound caller's own
    // `connect_timeout` fires first and surfaces as a generic
    // "operation timed out" error.
    let transient: IndexMap<String, String> = crate::session::Session::TRANSIENT_HEADER_KEYS
        .iter()
        .filter_map(|key| {
            let v = http_headers.get(*key)?.to_str().ok()?;
            Some((key.to_string(), v.to_string()))
        })
        .collect();

    let attempts = specs.into_iter().map(|spec| {
        let url = spec.url.clone();
        let headers_for_payload = spec.headers.clone();
        let transient = transient.clone();
        async move {
            // Hoist any caller-supplied `Mcp-Session-Id` out of the
            // header bag and pass it as the dedicated `session_id` arg.
            // Otherwise the upstream sees a session-id header but the
            // mcp client treats the connection as brand-new and
            // expects the server to mint a fresh id in the response;
            // when the upstream is already a pre-seeded rmcp session
            // (existing-session branch in rmcp's tower) the response
            // does NOT echo `Mcp-Session-Id`, so the client errors with
            // `NoSessionId` and retries forever.
            let mut headers = spec.headers;
            let session_id = headers.shift_remove(MCP_SESSION_ID_KEY);
            // Stamp the session-global transient headers on the initial
            // connect. Subsequent calls on this connection get them via
            // `Connection::extra_headers` (refreshed on every
            // `Session::apply_transient_headers`).
            for (k, v) in transient {
                headers.entry(k).or_insert(v);
            }
            let conn_result = client
                .connect(spec.url, session_id, Some(headers))
                .await;
            let conn = conn_result.map_err(|source| BadInit::UpstreamConnectFailed {
                url: url.clone(),
                source,
            })?;
            let payload_headers =
                build_canonical_headers(&headers_for_payload, &conn.session_id);
            Ok::<_, BadInit>((conn, payload_headers))
        }
    });

    let connections = try_join_all(attempts).await?;
    Ok(connections)
}

/// Reconnect to the upstreams encoded in a stale (decoded-but-not-
/// alive) session payload. Each URL gets connected with the headers
/// stored in the payload — the new request's `X-MCP-Servers` /
/// `X-MCP-Headers` are NOT consulted on this path. The encoded id is
/// the sole source of truth for what to reconnect to and how.
///
/// `Mcp-Session-Id` is pulled out of each per-URL header map and
/// passed to `Client::connect` as its dedicated `session_id` argument;
/// everything else (including `Authorization`) rides as `headers` and
/// gets stamped on every request the resulting `Connection` makes.
/// The returned pair includes the payload-derived header map, with
/// the `Mcp-Session-Id` refreshed to whatever the upstream returned
/// (which may be the same or a rotated sid).
pub async fn reconnect_from_payload(
    client: &Client,
    payload: &crate::session_manager::SessionPayload,
) -> Result<Vec<(Connection, IndexMap<String, String>)>, BadInit> {
    let attempts = payload.connections.iter().map(|(url, headers)| {
        let url = url.clone();
        let mut headers = headers.clone();
        let session_id = headers.shift_remove(MCP_SESSION_ID_KEY);
        // Agent identity headers live on `Session::transient_headers`
        // (extracted from the reconnect request's HeaderMap in
        // `handle_initialize`), not on the payload. The per-URL bag
        // here carries only `Authorization` + custom headers.
        let payload_headers = headers.clone();
        async move {
            let conn_result = client
                .connect(url.clone(), session_id, Some(headers))
                .await;
            let conn = conn_result.map_err(|source| BadInit::UpstreamConnectFailed {
                url: url.clone(),
                source,
            })?;
            let canonical = build_canonical_headers(&payload_headers, &conn.session_id);
            Ok::<_, BadInit>((conn, canonical))
        }
    });

    try_join_all(attempts).await
}

/// Build the canonical full header map for one upstream, suitable for
/// encoding into the session id. Sort happens later (in
/// `session_manager::build_payload`); this function just merges in the
/// freshly-minted `Mcp-Session-Id`.
fn build_canonical_headers(
    headers: &IndexMap<String, String>,
    upstream_session_id: &str,
) -> IndexMap<String, String> {
    let mut out: IndexMap<String, String> = headers.clone();
    out.insert(MCP_SESSION_ID_KEY.to_string(), upstream_session_id.to_string());
    out
}

fn parse_init_headers(
    http_headers: &HeaderMap,
) -> Result<Vec<UpstreamSpec>, BadInit> {
    let servers: Vec<String> = match http_headers.get(SERVERS_HEADER) {
        Some(v) => {
            let s = v.to_str().map_err(|_| BadInit::NotUtf8 { header: SERVERS_HEADER })?;
            serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                header: SERVERS_HEADER,
                source,
            })?
        }
        None => Vec::new(),
    };

    let mut per_url_headers: IndexMap<String, IndexMap<String, String>> =
        match http_headers.get(HEADERS_HEADER) {
            Some(v) => {
                let s = v.to_str().map_err(|_| BadInit::NotUtf8 { header: HEADERS_HEADER })?;
                serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                    header: HEADERS_HEADER,
                    source,
                })?
            }
            None => IndexMap::new(),
        };

    // Strip the session-global transient keys from every per-URL bag.
    // These keys live on `Session::transient_headers` (in-memory only,
    // never encoded into the session id) and re-stamp on every
    // outbound request via the SDK's `Connection.extra_headers`. A
    // caller-supplied per-URL entry for either key is dropped at parse
    // time so it can never leak into `SessionPayload.connections[url]`
    // or `Connection.headers`.
    for inner in per_url_headers.values_mut() {
        for key in crate::session::Session::TRANSIENT_HEADER_KEYS {
            inner.shift_remove(key);
        }
    }

    let mut seen = std::collections::HashSet::new();
    let mut specs = Vec::with_capacity(servers.len());
    for url in servers {
        // First-occurrence-wins de-duplication: a duplicate URL is silently
        // ignored. Prevents the proxy from opening N redundant upstream
        // connections to the same server when the client misconfigures.
        if !seen.insert(url.clone()) {
            tracing::debug!(url = %url, "ignoring duplicate X-MCP-Servers entry");
            continue;
        }

        let headers = per_url_headers.shift_remove(&url).unwrap_or_default();
        specs.push(UpstreamSpec { url, headers });
    }

    Ok(specs)
}
