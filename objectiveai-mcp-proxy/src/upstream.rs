//! Parsing of the proxy's two custom session-init headers and fan-out
//! connect over the resulting upstream specs.

use axum::http::HeaderMap;
use futures::TryFutureExt;
use futures::future::try_join_all;
use indexmap::IndexMap;
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use objectiveai_sdk::laboratories::Laboratory;
use objectiveai_sdk::mcp::Client;

use crate::reverse_channel::{ReverseChannel, Upstream};

const SERVERS_HEADER: &str = "X-MCP-Servers";
const HEADERS_HEADER: &str = "X-MCP-Headers";
/// Typed laboratory marker: JSON `{url: Laboratory}`. The authoritative
/// signal for which upstreams are laboratories — the proxy must NOT infer
/// laboratory-ness by string-parsing the `ws://laboratory/{id}` URL. An
/// upstream whose URL appears here is a laboratory, with its id taken from
/// the typed value (never the URL path).
const LABORATORIES_HEADER: &str = "X-MCP-Laboratories";
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
    /// Typed laboratory identity, from `X-MCP-Laboratories`. `Some` ⇒ the
    /// proxy treats this upstream as a laboratory (id taken from here, not
    /// from the URL).
    laboratory: Option<Laboratory>,
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
    /// An upstream's `initialize` handshake succeeded but a post-connect
    /// health probe (`tools/list` or `resources/list`) failed — the
    /// upstream accepted the connection but can't actually serve. Treated
    /// identically to a connect failure: the whole `initialize` fails.
    /// `kind` is `"tools"` or `"resources"`. No `#[source]` attribute:
    /// `Arc<Error>` doesn't impl `std::error::Error`, so the cause is
    /// folded into the Display message via the `Arc`'s deref.
    #[error("upstream {kind} list failed for {url}: {source}")]
    UpstreamListFailed {
        url: String,
        kind: &'static str,
        source: std::sync::Arc<objectiveai_sdk::mcp::Error>,
    },
}

/// HTTP header name used to carry the upstream MCP session id. When a
/// caller supplies it inside `X-MCP-Headers`, it's hoisted out of the
/// per-upstream header bag and passed to `Client::connect` as the
/// dedicated resume `session_id` argument.
pub const MCP_SESSION_ID_KEY: &str = "Mcp-Session-Id";

/// Parse the two custom session-init headers and fresh-connect to
/// every upstream URL they describe in parallel.
///
/// Every URL is connected from scratch. Sessions are keyed by the
/// objectiveai response id in `mcp::handle_initialize`, so there is no
/// resume-from-id path — a request for a known response id reuses the
/// live in-memory `Session`; an unknown one fresh-connects here.
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
/// Returns each opened [`Upstream`]. Duplicate URLs in `X-MCP-Servers`
/// are ignored (first-occurrence wins). If any upstream fails to
/// connect, the first such failure is returned as
/// `BadInit::UpstreamConnectFailed` and the remaining in-flight attempts
/// are dropped.
pub async fn connect_all_fresh(
    client: &Client,
    reverse_channel: Option<&ReverseChannel>,
    http_headers: &HeaderMap,
) -> Result<Vec<Upstream>, BadInit> {
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
        let transient = transient.clone();
        async move {
            // Hoist any caller-supplied `Mcp-Session-Id` out of the
            // header bag and pass it as the dedicated `session_id` arg
            // (HTTP) / replayed resume header (WS).
            let mut headers = spec.headers;
            let session_id = headers.shift_remove(MCP_SESSION_ID_KEY);
            // Stamp the session-global transient headers on the initial
            // connect. Subsequent calls get them via the upstream's
            // extra-headers bag (refreshed on every
            // `Session::apply_transient_headers`).
            for (k, v) in transient {
                headers.entry(k).or_insert(v);
            }
            let upstream = connect_upstream(
                client,
                reverse_channel,
                &url,
                session_id,
                headers,
                spec.laboratory,
            )
            .await?;
            // Health probe: the upstream must list both its tools and its
            // resources before we count it as connected. Run concurrently;
            // `try_join!` short-circuits on the first failure. `list_tools` /
            // `list_resources` return `Ok(empty)` when the matching
            // capability is absent, so a capability-less server passes and
            // only a genuine RPC/transport error fails the init.
            tokio::try_join!(
                upstream.list_tools().map_err(|source| BadInit::UpstreamListFailed {
                    url: url.clone(),
                    kind: "tools",
                    source,
                }),
                upstream.list_resources().map_err(|source| BadInit::UpstreamListFailed {
                    url: url.clone(),
                    kind: "resources",
                    source,
                }),
            )?;
            Ok::<_, BadInit>(upstream)
        }
    });

    try_join_all(attempts).await
}

/// Connect one upstream — HTTP via `client`, or `ws://` via the reverse
/// `channel` — returning the unified [`Upstream`]. `session_id` is the
/// resume `Mcp-Session-Id` (if any); `headers` is the per-upstream header
/// set already merged with the transient identity bag.
async fn connect_upstream(
    client: &Client,
    reverse_channel: Option<&ReverseChannel>,
    url: &str,
    session_id: Option<String>,
    mut headers: IndexMap<String, String>,
    laboratory: Option<Laboratory>,
) -> Result<Upstream, BadInit> {
    // Laboratory identity is authoritative from the explicit
    // `X-MCP-Laboratories` marker — its typed id drives `McpKind`, never
    // a parse of the URL path. Everything else falls back to URL-derived
    // kinds (objectiveai / plugin) or plain HTTP.
    let ws_kind = match &laboratory {
        Some(Laboratory::Client(c)) => Some(McpKind::Laboratory {
            id: c.id.clone(),
            // Laboratory ids are only unique per (machine, state) —
            // the marker's pair rides the kind so the CLI conduit
            // forwards to the exact host.
            machine: c.machine.clone(),
            machine_state: c.machine_state.clone(),
            agent: None,
        }),
        // Agent-embedded laboratory: no pinned (machine, state) — the
        // CLI conduit resolves (or creates) the laboratory from the
        // seed at Initialize.
        Some(Laboratory::Agent(a)) => Some(McpKind::Laboratory {
            id: a.id.clone(),
            machine: None,
            machine_state: None,
            agent: Some(
                objectiveai_sdk::client_objectiveai_mcp::AgentLaboratorySeed {
                    agent_full_id: a.agent_full_id.clone(),
                    laboratory: a.laboratory.clone(),
                },
            ),
        }),
        None => crate::reverse_channel::parse_ws_mcp_kind(url),
    };
    if let Some(mcp_kind) = ws_kind {
        let channel = reverse_channel.cloned().ok_or_else(|| {
            BadInit::UpstreamConnectFailed {
                url: url.to_string(),
                source: objectiveai_sdk::mcp::Error::MalformedResponse {
                    url: url.to_string(),
                    message: "ws:// upstream requires a reverse channel".into(),
                },
            }
        })?;
        // Resume: replay the stored upstream `Mcp-Session-Id` as a header
        // so the CLI conduit resumes that upstream's session.
        if let Some(sid) = session_id {
            headers.insert(MCP_SESSION_ID_KEY.to_string(), sid);
        }
        // Plugin args ride as the `X-OBJECTIVEAI-ARGUMENTS` per-upstream
        // header (JSON `{key: value|null}`), the same way the loopback path
        // carried them; lift them into the typed `InitializeRequest.args`
        // the CLI's `dial_plugin` reads. The header itself stays in
        // `headers` (later requests that touch the plugin env still read it).
        let args: IndexMap<String, Option<String>> = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-ARGUMENTS"))
            .and_then(|(_, v)| serde_json::from_str(v).ok())
            .unwrap_or_default();
        // Timeouts come from the per-request proxy config via the SDK
        // client: the connect timeout bounds the `initialize`, the call
        // timeout (per-request `X-MCP-CALL-TIMEOUT`; `None` = unbounded)
        // bounds every later op.
        let upstream = crate::reverse_channel::connect_ws(
            channel,
            url.to_string(),
            mcp_kind,
            args,
            headers,
            laboratory,
            client.connect_timeout,
            client.call_timeout,
        )
        .await
        .map_err(|source| BadInit::UpstreamConnectFailed {
            url: url.to_string(),
            source,
        })?;
        Ok(Upstream::Ws(upstream))
    } else {
        let conn = client
            .connect(url.to_string(), session_id, Some(headers))
            .await
            .map_err(|source| BadInit::UpstreamConnectFailed {
                url: url.to_string(),
                source,
            })?;
        Ok(Upstream::Http(conn))
    }
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

    // Typed laboratory marker, keyed by upstream URL. The authoritative
    // source for laboratory identity — joined to each spec by URL below.
    let mut laboratories: IndexMap<String, Laboratory> =
        match http_headers.get(LABORATORIES_HEADER) {
            Some(v) => {
                let s = v
                    .to_str()
                    .map_err(|_| BadInit::NotUtf8 { header: LABORATORIES_HEADER })?;
                serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                    header: LABORATORIES_HEADER,
                    source,
                })?
            }
            None => IndexMap::new(),
        };

    // Strip the session-global transient keys from every per-URL bag.
    // These keys live on `Session::transient_headers` (in-memory only)
    // and re-stamp on every outbound request via the SDK's
    // `Connection.extra_headers`. A caller-supplied per-URL entry for
    // any of these keys is dropped at parse time so it can never leak
    // into a `Connection`'s connect-time header set.
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
        let laboratory = laboratories.shift_remove(&url);
        specs.push(UpstreamSpec {
            url,
            headers,
            laboratory,
        });
    }

    Ok(specs)
}
