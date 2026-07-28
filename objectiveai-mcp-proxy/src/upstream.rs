//! Parsing of the proxy's two custom session-init headers and fan-out
//! connect over the resulting upstream specs.

use std::sync::Arc;

use axum::http::HeaderMap;
use futures::TryFutureExt;
use futures::future::try_join_all;
use indexmap::IndexMap;
use tokio::sync::RwLock;
use objectiveai_sdk::client_objectiveai_mcp::McpKind;
use objectiveai_sdk::laboratories::Laboratory;
use objectiveai_sdk::mcp::Client;

use crate::reverse_channel::{ReverseChannel, Upstream};

const SERVERS_HEADER: &str = "X-MCP-Servers";
const HEADERS_HEADER: &str = "X-MCP-Headers";
/// Typed laboratory marker: JSON `{url: Laboratory}`. The authoritative
/// signal for which upstreams are laboratories — the proxy must NOT infer
/// laboratory-ness by string-parsing the `client://laboratory/{id}` URL. An
/// upstream whose URL appears here is a laboratory, with its id taken from
/// the typed value (never the URL path).
const LABORATORIES_HEADER: &str = "X-MCP-Laboratories";
/// Typed plugin marker: JSON `{url: mcp::server::Plugin}`. The
/// authoritative signal for which upstreams are PLUGINS — the proxy
/// must NOT infer plugin-ness by string-parsing the
/// `client:///owner/name/version/mcp` URL (that parse is gone). An
/// upstream whose URL appears here is a plugin, with its four
/// coordinates taken from the typed value (never the URL path). A
/// plugin-marked `http(s)://` upstream (a future SERVER-SIDE plugin)
/// additionally connects with the proxy's REAL command executor.
const PLUGINS_HEADER: &str = "X-MCP-Plugins";
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
    /// Typed plugin identity, from `X-MCP-Plugins`. `Some` ⇒ the proxy
    /// treats this upstream as a plugin (coordinates taken from here,
    /// not from the URL).
    plugin: Option<objectiveai_sdk::mcp::server::Plugin>,
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
) -> Result<(Vec<Upstream>, Arc<RwLock<IndexMap<String, String>>>), BadInit> {
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
    // The SHARED transient bag: becomes `Session::transient_headers`
    // AND is handed to every plugin command executor, which reads it
    // at execute() time — so identity refreshed by a later
    // `initialize` reaches in-flight connections' executors too.
    let transient_shared = Arc::new(RwLock::new(transient.clone()));

    let attempts = specs.into_iter().map(|spec| {
        let url = spec.url.clone();
        let transient = transient.clone();
        let transient_shared = Arc::clone(&transient_shared);
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
                spec.plugin,
                transient_shared,
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

    let upstreams = try_join_all(attempts).await?;
    Ok((upstreams, transient_shared))
}

/// Connect one upstream — HTTP via `client`, or `client://` via the reverse
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
    plugin: Option<objectiveai_sdk::mcp::server::Plugin>,
    transient: Arc<RwLock<IndexMap<String, String>>>,
) -> Result<Upstream, BadInit> {
    // Laboratory and plugin identities are authoritative from their
    // explicit typed markers (`X-MCP-Laboratories` / `X-MCP-Plugins`)
    // — never a parse of the URL path. An unmarked `client://` URL is
    // a hard error; everything else is plain HTTP.
    let ws_kind = match &laboratory {
        Some(Laboratory::Client(c)) => Some(McpKind::Laboratory {
            id: c.id.clone(),
            // Laboratory ids are only unique per (machine, state) —
            // the marker's pair rides the kind so the CLI conduit
            // forwards to the exact host.
            machine: c.machine.clone(),
            machine_state: c.machine_state.clone(),
        }),
        // Agent-embedded laboratory: no pinned (machine, state) — the
        // CLI conduit resolves (or creates) the laboratory from the
        // seed at Initialize.
        Some(Laboratory::Agent(a)) => Some(McpKind::AgentLaboratory {
            id: a.id.clone(),
            agent_full_id: a.agent_full_id.clone(),
            laboratory: a.laboratory.clone(),
        }),
        // A client:// upstream is either a marked plugin (client-side
        // plugin — the typed marker supplies the coordinates) or a
        // hard error: kinds are never URL-inferred.
        None if url.starts_with("client://") => match &plugin {
            Some(p) => Some(McpKind::PluginLaboratory {
                owner: p.owner.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
            }),
            None => {
                return Err(BadInit::UpstreamConnectFailed {
                    url: url.to_string(),
                    source: objectiveai_sdk::mcp::Error::MalformedResponse {
                        url: url.to_string(),
                        message: "client:// upstream requires an \
                                  X-MCP-Plugins or X-MCP-Laboratories \
                                  marker"
                            .into(),
                    },
                });
            }
        },
        None => None,
    };
    if let Some(mcp_kind) = ws_kind {
        let channel = reverse_channel.cloned().ok_or_else(|| {
            BadInit::UpstreamConnectFailed {
                url: url.to_string(),
                source: objectiveai_sdk::mcp::Error::MalformedResponse {
                    url: url.to_string(),
                    message: "client:// upstream requires a reverse channel".into(),
                },
            }
        })?;
        // Resume: replay the stored upstream `Mcp-Session-Id` as a header
        // so the CLI conduit resumes that upstream's session.
        if let Some(sid) = session_id {
            headers.insert(MCP_SESSION_ID_KEY.to_string(), sid);
        }
        // Plugin args ride as the `X-OBJECTIVEAI-ARGUMENTS` per-upstream
        // header (JSON `{key: value|null}`) and stay HEADERS-ONLY: the
        // header flows through the whole forward chain to the plugin
        // container's MCP server — nothing lifts it into typed params.
        //
        // Timeouts come from the per-request proxy config via the SDK
        // client: the connect timeout bounds the `initialize`, the call
        // timeout (per-request `X-MCP-CALL-TIMEOUT`; `None` = unbounded)
        // bounds every later op.
        let upstream = crate::reverse_channel::connect_ws(
            channel,
            url.to_string(),
            mcp_kind,
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
    } else if let Some(plugin) = plugin {
        // A plugin-marked http(s) upstream: a SERVER-SIDE plugin.
        // Connect with the proxy's REAL command executor so the
        // plugin's `cli_request` frames are fulfilled by the daemon
        // over the reverse channel. No channel (standalone proxy) →
        // hard error: a server-side plugin without a daemon to run
        // its commands is a misconfiguration, not a downgrade.
        let Some(channel) = reverse_channel else {
            return Err(BadInit::UpstreamConnectFailed {
                url: url.to_string(),
                source: objectiveai_sdk::mcp::Error::MalformedResponse {
                    url: url.to_string(),
                    message: "plugin-marked upstream requires a reverse \
                              channel"
                        .into(),
                },
            });
        };
        let executor = crate::command_executor::ReverseChannelCommandExecutor {
            channel: channel.clone(),
            plugin: plugin.clone(),
            transient,
            ack_timeout: client.call_timeout,
        };
        let connection = client
            .clone()
            .with_executor(executor)
            .connect(url.to_string(), session_id, Some(headers))
            .await
            .map_err(|source| BadInit::UpstreamConnectFailed {
                url: url.to_string(),
                source,
            })?;
        Ok(Upstream::HttpPlugin { connection, plugin })
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

    // Typed plugin marker, keyed by upstream URL. Same authority rule
    // as laboratories: the marker, never the URL.
    let mut plugins: IndexMap<String, objectiveai_sdk::mcp::server::Plugin> =
        match http_headers.get(PLUGINS_HEADER) {
            Some(v) => {
                let s = v
                    .to_str()
                    .map_err(|_| BadInit::NotUtf8 { header: PLUGINS_HEADER })?;
                serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                    header: PLUGINS_HEADER,
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
        let plugin = plugins.shift_remove(&url);
        specs.push(UpstreamSpec {
            url,
            headers,
            laboratory,
            plugin,
        });
    }

    Ok(specs)
}
