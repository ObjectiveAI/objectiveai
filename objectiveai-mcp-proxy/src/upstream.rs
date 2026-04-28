//! Parsing of the proxy's three custom session-init headers and fan-out
//! connect over the resulting upstream specs.

use axum::http::HeaderMap;
use futures::future::try_join_all;
use indexmap::IndexMap;
use objectiveai::mcp::{Client, Connection};

const SERVERS_HEADER: &str = "X-MCP-Servers";
const HEADERS_HEADER: &str = "X-MCP-Headers";
const AUTHORIZATION_HEADER: &str = "X-MCP-Authorization";
const AUTHORIZATION_KEY: &str = "Authorization";

/// One upstream MCP server the proxy should connect to for a session.
#[derive(Debug)]
struct UpstreamSpec {
    url: String,
    authorization: Option<String>,
    extra_headers: IndexMap<String, String>,
}

/// Why parsing the three custom session-init headers failed, or why an
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
        source: objectiveai::mcp::Error,
    },
}

/// Parse the three custom session-init headers and connect to every
/// upstream they describe in parallel.
///
/// Headers (all optional):
/// - `X-MCP-Servers`: JSON array of upstream URLs. Empty / absent → empty
///   Vec is returned (the session still initializes, the client just gets
///   nothing from `tools/list` etc).
/// - `X-MCP-Headers`: JSON `{string: string}` of extra HTTP headers
///   forwarded on every upstream request.
/// - `X-MCP-Authorization`: JSON `{url: string}` per-URL `Authorization`
///   value. Overrides whatever `X-MCP-Headers` would have sent for that URL.
///
/// Duplicate URLs in `X-MCP-Servers` are ignored (first-occurrence wins).
/// If any upstream fails to connect, the first such failure is returned
/// as `BadInit::UpstreamConnectFailed` and the remaining in-flight
/// attempts are dropped. The returned Vec preserves the order URLs first
/// appeared in `X-MCP-Servers`.
pub async fn connect_all(
    client: &Client,
    http_headers: &HeaderMap,
) -> Result<Vec<Connection>, BadInit> {
    let specs = parse_init_headers(http_headers)?;

    let attempts = specs.into_iter().map(|spec| {
        let url = spec.url.clone();
        async move {
            client
                .connect(spec.url, spec.authorization, None, spec.extra_headers)
                .await
                .map_err(|source| BadInit::UpstreamConnectFailed { url, source })
        }
    });

    try_join_all(attempts).await
}

fn parse_init_headers(http_headers: &HeaderMap) -> Result<Vec<UpstreamSpec>, BadInit> {
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

    let global_headers: IndexMap<String, String> = match http_headers.get(HEADERS_HEADER) {
        Some(v) => {
            let s = v.to_str().map_err(|_| BadInit::NotUtf8 { header: HEADERS_HEADER })?;
            serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                header: HEADERS_HEADER,
                source,
            })?
        }
        None => IndexMap::new(),
    };

    let per_url_authorization: IndexMap<String, String> =
        match http_headers.get(AUTHORIZATION_HEADER) {
            Some(v) => {
                let s = v.to_str().map_err(|_| BadInit::NotUtf8 {
                    header: AUTHORIZATION_HEADER,
                })?;
                serde_json::from_str(s).map_err(|source| BadInit::NotJson {
                    header: AUTHORIZATION_HEADER,
                    source,
                })?
            }
            None => IndexMap::new(),
        };

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

        let mut headers = global_headers.clone();
        // Lift Authorization out of the global header set into its dedicated
        // field; per-URL override wins if present.
        let authorization = per_url_authorization
            .get(&url)
            .cloned()
            .or_else(|| headers.shift_remove(AUTHORIZATION_KEY));

        specs.push(UpstreamSpec {
            url,
            authorization,
            extra_headers: headers,
        });
    }

    Ok(specs)
}
