//! Shared plumbing for talking to a laboratory container's MCP server
//! over its published loopback port — used by BOTH laboratory kinds
//! (the regular [`LabServer`](crate::server::LabServer) and the
//! [`EphemeralLab`](crate::ephemeral::EphemeralLab)).

use indexmap::IndexMap;
use objectiveai_sdk::laboratories::daemon::JsonRpcResult;

pub(crate) fn rpc_err<T>(code: i64, message: String) -> JsonRpcResult<T> {
    JsonRpcResult::Err { code, message, data: None }
}

pub(crate) fn response_id_from_headers(
    headers: &IndexMap<String, String>,
) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("X-OBJECTIVEAI-RESPONSE-ID"))
        .map(|(_, v)| v.clone())
}

pub(crate) fn sanitize_connect_headers(
    headers: &IndexMap<String, String>,
) -> IndexMap<String, String> {
    let mut out = headers.clone();
    for k in [
        "Host",
        "host",
        "Content-Length",
        "content-length",
        "Mcp-Session-Id",
        "mcp-session-id",
    ] {
        out.shift_remove(k);
    }
    out
}

/// The MCP client every laboratory connection uses — the CLI conduit's
/// knobs (100ms/0.5/1.5/1s backoff, 10-minute budget + call timeout;
/// the laboratory is loopback, so these are generous). Lives here so
/// the knobs exist ONCE for both laboratory kinds.
pub(crate) fn lab_mcp_client() -> objectiveai_sdk::mcp::Client {
    use std::time::Duration;
    objectiveai_sdk::mcp::Client::new(
        reqwest::Client::new(),
        "objectiveai-laboratory".to_string(),
        String::new(),
        String::new(),
        Some(Duration::from_secs(600)),
        Duration::from_millis(100),
        Duration::from_millis(100),
        0.5,
        1.5,
        Duration::from_millis(1000),
        Duration::from_secs(600),
        Some(Duration::from_secs(600)),
    )
}

/// Raw JSON-RPC POST through an `mcp::Connection` — the conduit's
/// `upstream_call`, with a plain-`String` error.
pub(crate) async fn raw_call<P, R>(
    conn: &objectiveai_sdk::mcp::Connection<crate::host_command::HostCommandExecutor>,
    headers: &IndexMap<String, String>,
    method: &str,
    params: &P,
) -> Result<JsonRpcResult<R>, String>
where
    P: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let rpc_id = uuid::Uuid::new_v4().to_string();
    let envelope = serde_json::json!({
        "jsonrpc": "2.0",
        "id": rpc_id,
        "method": method,
        "params": params,
    });

    let mut req = conn.http_client.post(&conn.url);
    for (k, v) in headers {
        if k.eq_ignore_ascii_case("host")
            || k.eq_ignore_ascii_case("content-length")
            || k.eq_ignore_ascii_case("connection")
            || k.eq_ignore_ascii_case("accept")
            || k.eq_ignore_ascii_case("content-type")
            || k.eq_ignore_ascii_case("mcp-session-id")
        {
            continue;
        }
        req = req.header(k, v);
    }
    req = req
        .header("Content-Type", "application/json")
        .header("Accept", "application/json, text/event-stream")
        .header("Mcp-Session-Id", &conn.session_id)
        .json(&envelope);

    let resp = req
        .send()
        .await
        .map_err(|e| format!("forwarding HTTP request failed: {e}"))?;
    let resp_text = resp
        .text()
        .await
        .map_err(|e| format!("reading response body failed: {e}"))?;
    let Some(body) = parse_json_or_sse(&resp_text) else {
        return Err("empty or unparseable upstream response".into());
    };

    if let Some(result) = body.get("result") {
        let typed: R = serde_json::from_value(result.clone())
            .map_err(|e| format!("decode upstream result: {e}"))?;
        return Ok(JsonRpcResult::Ok { result: typed });
    }
    if let Some(err) = body.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(-32603);
        let message = err
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("upstream returned an error envelope without a message")
            .to_string();
        let data = err.get("data").cloned();
        return Ok(JsonRpcResult::Err { code, message, data });
    }
    Err("upstream response missing both `result` and `error`".into())
}

/// Streamable-HTTP responses may be plain JSON or an SSE body whose
/// `data:` lines carry the JSON — accept both.
fn parse_json_or_sse(text: &str) -> Option<serde_json::Value> {
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
        return Some(v);
    }
    let collected: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: ").or_else(|| l.strip_prefix("data:")))
        .collect();
    if collected.is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&collected).ok()
}
