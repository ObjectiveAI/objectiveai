//! MCP Streamable-HTTP endpoints. POST handles JSON-RPC requests +
//! notifications + responses; GET serves the server-initiated SSE stream.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Bytes,
    extract::{Json, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{
        IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::{StreamExt, stream};
use objectiveai_sdk::mcp::{
    JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
    initialize_result::{
        Implementation, InitializeResult, ResourcesCapability, ServerCapabilities,
        ToolsCapability,
    },
    resource::ReadResourceRequestParams,
    tool::{CallToolRequestParams, ContentBlock, TextContent},
};
use tokio::sync::broadcast;

use crate::AppState;
use crate::session::{CallToolError, ReadResourceError, Session};
use crate::session_manager::SessionManager;
use crate::upstream::BadInit;

/// MCP protocol version the proxy advertises in its `initialize`
/// response. Pinned — the proxy implements 2025-06-18 semantics.
const PROTOCOL_VERSION: &str = "2025-06-18";

/// Versions the proxy is willing to accept on incoming `initialize`
/// requests. Per the MCP spec, the server picks one of these (the
/// proxy's pinned [`PROTOCOL_VERSION`]) and the client downgrades. The
/// `@modelcontextprotocol/sdk` TypeScript client defaults to
/// `2025-11-25`; including it here lets that client connect without
/// needing the SDK to pre-negotiate.
const ACCEPTED_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-11-25"];

/// JSON-RPC error codes we use.
const PARSE_ERROR: i64 = -32700;
const INVALID_REQUEST: i64 = -32600;
const METHOD_NOT_FOUND: i64 = -32601;
const INVALID_PARAMS: i64 = -32602;
const INTERNAL_ERROR: i64 = -32603;
/// Spec-defined "Request cancelled" code per MCP 2025-11-25 utilities/cancellation.
const REQUEST_CANCELLED: i64 = -32800;

/// Header the client sends to identify its session on every request after
/// `initialize`.
const SESSION_ID_HEADER: &str = "Mcp-Session-Id";

/// SSE keepalive cadence — picks something well under typical proxy /
/// load balancer idle timeouts.
const SSE_KEEP_ALIVE: Duration = Duration::from_secs(15);

// ---- POST handler ----------------------------------------------------------

/// POST `/`: receive a single JSON-RPC envelope, dispatch by method.
///
/// **Cancellation propagation note.** The handler future is held directly
/// by axum (no `tokio::spawn` between us and the upstream RPC). When the
/// downstream client closes its TCP connection mid-request, axum drops
/// this future, which drops the in-flight `session.call_tool(&params).await`,
/// which drops reqwest's `send()` future, which sends RST_STREAM (HTTP/2)
/// or closes the TCP stream (HTTP/1.1). That gives us connection-level
/// cancellation propagation for free.
pub async fn handle_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let session_in = headers
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if let Err(resp) = require_streamable_http_accept(&headers) {
        return resp;
    }

    // Manual body parse so malformed JSON returns a JSON-RPC -32700
    // envelope rather than axum's plain-text 400.
    let body: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return parse_error_response(format!("invalid JSON: {e}")),
    };

    // Notifications (no `id`) and responses get 202 Accepted with no body.
    // notifications/cancelled is the one notification we actually act on:
    // look up the in-flight token for params.requestId in the session
    // (if any) and fire it.
    if body.get("id").is_none() {
        if let Ok(notification) =
            serde_json::from_value::<JsonRpcNotification>(body)
        {
            if notification.method == "notifications/cancelled" {
                handle_cancelled_notification(&state, &headers, &notification);
            }
        }
        return StatusCode::ACCEPTED.into_response();
    }

    let request: JsonRpcRequest = match serde_json::from_value(body) {
        Ok(r) => r,
        Err(e) => return parse_error_response(format!("invalid JSON-RPC envelope: {e}")),
    };

    let method = request.method.clone();
    let rpc_id_str = format!("{}", request.id);
    let response = match request.method.as_str() {
        "initialize" => handle_initialize(&state, &headers, request).await,
        "ping" => handle_ping(request),
        "tools/list" => handle_tools_list(&state.sessions, &headers, request).await,
        "tools/call" => handle_tools_call(&state.sessions, &headers, request).await,
        "resources/list" => handle_resources_list(&state.sessions, &headers, request).await,
        "resources/read" => handle_resources_read(&state.sessions, &headers, request).await,
        other => method_not_found_response(request.id, other),
    };
    response
}

/// Look up the in-flight token for `params.requestId` and fire it. Quietly
/// no-ops if anything's missing — there's nothing to do if we can't
/// identify the request being cancelled.
fn handle_cancelled_notification(
    state: &AppState,
    headers: &HeaderMap,
    notification: &JsonRpcNotification,
) {
    let session_id = match headers
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => return,
    };
    let session = match state.sessions.get(session_id) {
        Some(s) => s,
        None => return,
    };
    let request_id = match notification
        .params
        .as_ref()
        .and_then(|p| p.get("requestId"))
    {
        Some(id) => id,
        None => return,
    };
    let cancelled = session.cancel_in_flight(request_id);
    tracing::debug!(
        session = %session_id,
        request_id = %request_id,
        cancelled,
        "notifications/cancelled received",
    );
}

// ---- DELETE handler (explicit session termination) ------------------------

/// DELETE `/`: end the session named by `Mcp-Session-Id`. Per
/// 2025-06-18/basic/transports#session-management the client uses this to
/// explicitly terminate its session; the server responds 200 on success
/// and 404 if the id is unknown.
pub async fn handle_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let session_id = match extract_session_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    match state.sessions.remove(&session_id) {
        Some(_) => StatusCode::OK.into_response(),
        None => (StatusCode::NOT_FOUND, "unknown session").into_response(),
    }
}

// ---- GET handler (server-initiated SSE stream) ----------------------------

/// GET `/`: open the per-session SSE stream so the server can push
/// server-initiated notifications to the client. Currently sources from
/// `session.outbound`, which the upstream-list-changed callbacks publish
/// `notifications/tools/list_changed` and
/// `notifications/resources/list_changed` onto. Periodic SSE keepalives
/// hold the connection open during quiet periods.
pub async fn handle_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let session_id = match extract_session_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.get(&session_id) {
        Some(s) => s,
        None => return (StatusCode::NOT_FOUND, "unknown session").into_response(),
    };

    let rx = session.outbound.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(notification) => {
                    let event = match Event::default().json_data(&notification) {
                        Ok(e) => e,
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to encode SSE event");
                            continue;
                        }
                    };
                    return Some((Ok::<_, Infallible>(event), rx));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(skipped = n, "SSE consumer lagged; dropped notifications");
                    continue;
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(SSE_KEEP_ALIVE))
        .into_response()
}

// ---- Method handlers ------------------------------------------------------

async fn handle_initialize(
    state: &AppState,
    headers: &HeaderMap,
    request: JsonRpcRequest,
) -> Response {
    let session_in = headers
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let rpc_id_str = format!("{}", request.id);
    // Validate the client's requested protocolVersion. We don't care
    // about anything else in `params` (clientInfo / capabilities) — they
    // don't change our routing or our advertised feature set.
    match request.params.as_ref().and_then(|p| p.get("protocolVersion")) {
        Some(v) => match v.as_str() {
            Some(version) if ACCEPTED_PROTOCOL_VERSIONS.contains(&version) => {
                // Accepted; the response will downgrade to PROTOCOL_VERSION
                // and a spec-compliant client adopts it.
            }
            Some(other) => {
                return invalid_request_response(
                    request.id,
                    format!(
                        "unsupported protocolVersion {other:?}; this proxy accepts {ACCEPTED_PROTOCOL_VERSIONS:?}",
                    ),
                );
            }
            None => {
                return invalid_params_response(
                    request.id,
                    "params.protocolVersion must be a string".into(),
                );
            }
        },
        None => {
            return invalid_params_response(
                request.id,
                "params.protocolVersion is required".into(),
            );
        }
    }

    // Proxy session ids are AEAD-encrypted, base62-encoded envelopes
    // wrapping a `URL → header_map` payload. The header map is the
    // full set of HTTP headers needed to reconnect that upstream —
    // `Mcp-Session-Id`, `Authorization`, plus any custom `X-*`. The
    // upstream session id is uniform with every other header, no
    // dedicated field. See `session_manager::SessionPayload`.
    //
    // Three branches:
    //   1. id provided + alive in `state.sessions` → cheap-path: reuse
    //      the live in-memory `Session`, re-mint its id from its
    //      stored payload. The new request's `X-MCP-Servers` /
    //      `X-MCP-Headers` are IGNORED — the encoded id is the sole
    //      source of truth for what's connected and how. This is the
    //      "one id = one connection state" guarantee.
    //   2. id provided but not in memory → decrypt-and-decode it. If
    //      decryption fails, or decoding fails → 401. Otherwise
    //      reconnect to every URL in the decoded payload using its
    //      stored headers (same ignore-the-request semantics as
    //      branch 1). The `Mcp-Session-Id` and `Authorization`
    //      headers come ONLY from the encoded id; anything the
    //      request snuck into `X-MCP-Headers` is ignored.
    //   3. no id → fresh init. `X-MCP-Servers` / `X-MCP-Headers`
    //      build the spec list, every URL connects from scratch, the
    //      resulting `(Connection, headers)` set encodes into a
    //      brand-new id.
    let provided_session_id = headers
        .get(SESSION_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);

    if let Some(sid) = &provided_session_id {
        if let Some(session) = state.sessions.get(sid) {
            // Branch 1 — alive in-memory. Re-mint to keep the
            // in-memory session-key consistent (deterministic AEAD
            // nonce per `41c90d61` makes it byte-identical to what
            // the client holds), but DON'T echo the id back: per
            // rmcp's resume behavior we omit the `Mcp-Session-Id`
            // response header.
            let new_id = state.sessions.mint_id(&session.payload);
            let _ = (new_id, session);
            return ok_response_resume_sse(request.id);
        }
        // Branch 2 — decrypt and reconnect strictly from the payload.
        let (connections_with_headers, decoded_agent_id, decoded_tool_allowlists) = match state.sessions.decode_session_id(sid) {
            Some(payload) => {
                let agent_id = payload.agent_id.clone();
                let tool_allowlists = payload.tool_allowlists.clone();
                match crate::upstream::reconnect_from_payload(&state.client, &payload).await {
                    Ok(pairs) => (pairs, agent_id, tool_allowlists),
                    Err(e @ BadInit::UpstreamConnectFailed { .. }) => {
                        return internal_error_response(request.id, e.to_string());
                    }
                    Err(e) => {
                        // NotUtf8 / NotJson can't fire on this path
                        // (no header parsing) but exhaustively handled.
                        return internal_error_response(request.id, e.to_string());
                    }
                }
            }
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    format!("Unauthorized: Session not found ({sid:?})"),
                )
                    .into_response();
            }
        };
        // Register the just-reconnected session in memory. The minted
        // id is deterministic and matches what the client already
        // holds; we discard the return value because the resume path
        // doesn't echo the id back.
        let _ = state.sessions.add(
            connections_with_headers,
            decoded_agent_id,
            decoded_tool_allowlists,
        );
        ok_response_resume_sse(request.id)
    } else {
        // Branch 3 — fresh init. `X-MCP-Servers` / `X-MCP-Headers`
        // build the spec list, every URL connects from scratch, the
        // resulting `(Connection, headers)` set encodes into a
        // brand-new id which we echo back in the response header +
        // SSE-deliver the `InitializeResult`.
        // Capture the caller-supplied agent id at session-open. The
        // value rides inside the encrypted session id we mint below,
        // is recoverable from `session.payload.agent_id` on every
        // subsequent call, AND gets stamped on every outbound request
        // each upstream connection makes (via connect_all_fresh below).
        let agent_id = headers
            .get("X-OBJECTIVEAI-AGENT-ID")
            .or_else(|| headers.get("OBJECTIVEAI-AGENT-ID"))
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let (connections_with_headers, tool_allowlists) = match crate::upstream::connect_all_fresh(
            &state.client,
            headers,
            agent_id.as_deref(),
        ).await {
            Ok(pair) => pair,
            Err(e @ (BadInit::NotUtf8 { .. } | BadInit::NotJson { .. })) => {
                return invalid_request_response(request.id, e.to_string());
            }
            Err(e @ BadInit::UpstreamConnectFailed { .. }) => {
                return internal_error_response(request.id, e.to_string());
            }
        };
        let session_id = state.sessions.add(connections_with_headers, agent_id, tool_allowlists);
        ok_response_fresh_sse(request.id, session_id)
    }
}

/// Fresh-init `initialize` response: 200 OK + `Mcp-Session-Id`
/// response header + an SSE stream emitting one `data:` event
/// carrying the `InitializeResult` JSON, then closing.
///
/// Streamable-HTTP MCP servers reply over SSE when the client's
/// `Accept` lists `text/event-stream` (`require_streamable_http_accept`
/// already 406s callers that don't list both `application/json` and
/// SSE). rmcp's reference server uses SSE for the initialize reply
/// too; matching that shape is what keeps `claude_agent_sdk`'s
/// bundled CLI from silently filtering every tool from this server
/// out of the model's catalog.
fn ok_response_fresh_sse(
    request_id: serde_json::Value,
    session_id: String,
) -> Response {
    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.into(),
        capabilities: server_capabilities(),
        server_info: server_info(),
        instructions: None,
        _meta: None,
    };
    let body: JsonRpcResponse<InitializeResult> = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request_id,
        result,
    };
    let payload = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(e) => {
            return internal_error_response(
                serde_json::Value::Null,
                format!("failed to serialize InitializeResult: {e}"),
            );
        }
    };

    let header_value = match HeaderValue::from_str(&session_id) {
        Ok(v) => v,
        Err(_) => {
            return internal_error_response(
                serde_json::Value::Null,
                format!("session id is not a valid header value: {session_id}"),
            );
        }
    };

    let stream = stream::once(async move {
        Ok::<sse_stream::Sse, Infallible>(sse_stream::Sse::default().data(payload))
    });
    let body_stream = sse_stream::SseBody::new(stream);

    let mut response = Response::new(axum::body::Body::new(body_stream));
    *response.status_mut() = StatusCode::OK;
    let h = response.headers_mut();
    h.insert(SESSION_ID_HEADER, header_value);
    h.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    h.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

/// Resume-init `initialize` response: the client already carries an
/// `Mcp-Session-Id` header for a session we recognize (Branch 1
/// alive) or one we just decoded and reconnected (Branch 2). Matches
/// `rmcp`'s reference behavior — yields two SSE events, then leaves
/// the stream open until the client disconnects:
///
///   1. A priming event (`data: \nid: 0\nretry: 3000\n\n`).
///      `axum::response::sse::Event::data("")` short-circuits empty
///      input and never writes the `data:` prefix at all, so we use
///      `sse-stream` (the same crate `rmcp` uses) which writes
///      `data:\n` even for empty payloads. SSE clients — including
///      `claude_agent_sdk`'s bundled CLI — ignore events without a
///      `data:` line.
///   2. The `InitializeResult` JSON as a `data:` event, so the
///      client gets the result it asked for, just like a fresh init.
///
/// No `Mcp-Session-Id` response header (the client already holds
/// the id). Echoing back another `InitializeResult` via plain JSON
/// (which is what this function did before) is what made
/// `claude_agent_sdk`'s bundled CLI treat the resume as
/// protocol-noncompliant and drop every tool from this server.
fn ok_response_resume_sse(request_id: serde_json::Value) -> Response {
    let priming = sse_stream::Sse::default()
        .data("")
        .id("0")
        .retry_duration(Duration::from_millis(3000));

    let result = InitializeResult {
        protocol_version: PROTOCOL_VERSION.into(),
        capabilities: server_capabilities(),
        server_info: server_info(),
        instructions: None,
        _meta: None,
    };
    let body: JsonRpcResponse<InitializeResult> = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request_id,
        result,
    };
    let payload = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(e) => {
            return internal_error_response(
                serde_json::Value::Null,
                format!("failed to serialize InitializeResult: {e}"),
            );
        }
    };
    let result_event = sse_stream::Sse::default().data(payload);

    let stream = stream::iter(vec![
        Ok::<sse_stream::Sse, Infallible>(priming),
        Ok(result_event),
    ])
    .chain(stream::pending::<Result<sse_stream::Sse, Infallible>>());
    let body_stream = sse_stream::SseBody::new(stream);

    let mut response = Response::new(axum::body::Body::new(body_stream));
    *response.status_mut() = StatusCode::OK;
    let h = response.headers_mut();
    h.insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    h.insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

fn handle_ping(request: JsonRpcRequest) -> Response {
    let body: JsonRpcResponse<serde_json::Value> = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id: request.id,
        result: serde_json::json!({}),
    };
    (StatusCode::OK, Json(body)).into_response()
}

async fn handle_tools_list(
    sessions: &SessionManager,
    headers: &HeaderMap,
    request: JsonRpcRequest,
) -> Response {
    let session_id = match extract_session_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    // Optional `X-List-Filter`: scope the fan-out to a single
    // upstream URL. Absent → fan out to every upstream. Same header
    // applies to both `tools/list` and `resources/list`.
    let filter_url = headers
        .get(crate::upstream::LIST_FILTER_HEADER)
        .and_then(|v| v.to_str().ok());

    match session.list_tools_filtered(filter_url).await {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id: request.id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => internal_error_response(request.id, format!("list_tools: {e}")),
    }
}

async fn handle_tools_call(
    sessions: &SessionManager,
    headers: &HeaderMap,
    request: JsonRpcRequest,
) -> Response {
    let session_id = match extract_session_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let params: CallToolRequestParams = match request.params.clone() {
        Some(v) => match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => {
                return invalid_params_response(
                    request.id,
                    format!("tools/call params: {e}"),
                );
            }
        },
        None => return invalid_params_response(request.id, "missing params".into()),
    };

    let token = session.register_in_flight(&request.id);
    let _guard = InFlightGuard {
        session: Arc::clone(&session),
        id: request.id.clone(),
    };

    let result = tokio::select! {
        biased;
        _ = token.cancelled() => {
            return cancelled_response(request.id);
        }
        result = session.call_tool(&params) => result,
    };

    match result {
        Ok(mut result) => {
            // Drain any pending `/notify` content blocks and prepend
            // them, wrapped in a `<system-reminder>` text-block pair,
            // ahead of the upstream's tool-result content. Anything
            // queued after this drain rides the *next* tool call.
            let pending = session.drain_notifications().await;
            if !pending.is_empty() {
                let mut prefixed = Vec::with_capacity(2 + pending.len() + result.content.len());
                prefixed.push(ContentBlock::Text(TextContent {
                    text: SYSTEM_REMINDER_PREFIX.to_string(),
                    annotations: None,
                    _meta: None,
                }));
                prefixed.extend(pending);
                prefixed.push(ContentBlock::Text(TextContent {
                    text: SYSTEM_REMINDER_SUFFIX.to_string(),
                    annotations: None,
                    _meta: None,
                }));
                prefixed.append(&mut result.content);
                result.content = prefixed;
            }
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id: request.id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(CallToolError::ToolNotFound(name)) => {
            method_not_found_response(request.id, &format!("tool: {name}"))
        }
        Err(CallToolError::Upstream(e)) => {
            internal_error_response(request.id, format!("upstream call_tool: {e}"))
        }
    }
}

/// Wrap text bracketing `pending_notifications` blocks on the next
/// `tools/call` response. Mirrors the way Claude itself surfaces
/// in-flight user messages — text-only, no `IMPORTANT:` line.
const SYSTEM_REMINDER_PREFIX: &str =
    "<system-reminder>\nThe user sent a new message while you were working:\n";
const SYSTEM_REMINDER_SUFFIX: &str = "\n\n</system-reminder>";

/// `POST /notify` — queue content blocks to be prepended (wrapped in a
/// `<system-reminder>` block) onto the next `tools/call` response on
/// the session named by `Mcp-Session-Id`.
pub async fn handle_notify(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let session_id = match extract_session_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let blocks: Vec<ContentBlock> = match serde_json::from_slice(&body) {
        Ok(b) => b,
        Err(e) => {
            return parse_error_response(format!("invalid /notify body: {e}"));
        }
    };

    session.enqueue_notifications(blocks).await;
    StatusCode::ACCEPTED.into_response()
}

/// `GET /notify` — atomically drain the pending-notifications queue for
/// the session named by `Mcp-Session-Id` and return the blocks as a
/// JSON array. Used by the agent completions client at the start of a
/// new turn to pick up notifications that would otherwise have ridden
/// the next `tools/call` response — covering the case where the prior
/// turn ended without a tool call, or the user is starting a fresh
/// continuation.
///
/// Same drain semantics as the tool-response path: a second `GET /notify`
/// returns `[]` until the next `POST /notify`.
pub async fn handle_notify_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let session_id = match extract_session_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let blocks = session.drain_notifications().await;
    (StatusCode::OK, Json(blocks)).into_response()
}

/// `GET /notify/queued` — non-draining peek at the pending-notifications
/// queue for the session named by `Mcp-Session-Id`. Returns a bare JSON
/// boolean: `true` iff there is at least one queued block, `false`
/// otherwise. The queue is left untouched, so subsequent
/// `GET /notify` (drain) calls still see everything.
///
/// Used by the agent completions client to annotate the final chunk
/// with `messages_queued` so the caller knows whether a follow-up
/// continuation is needed to flush queued blocks.
pub async fn handle_notify_queued_get(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let session_id = match extract_session_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let queued = session.has_pending_notifications().await;
    (StatusCode::OK, Json(queued)).into_response()
}

async fn handle_resources_list(
    sessions: &SessionManager,
    headers: &HeaderMap,
    request: JsonRpcRequest,
) -> Response {
    let session_id = match extract_session_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    // Optional `X-List-Filter`: scope the fan-out to a single
    // upstream URL. Absent → fan out to every upstream. Same header
    // semantics as `tools/list`.
    let filter_url = headers
        .get(crate::upstream::LIST_FILTER_HEADER)
        .and_then(|v| v.to_str().ok());

    match session.list_resources_filtered(filter_url).await {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id: request.id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => internal_error_response(request.id, format!("list_resources: {e}")),
    }
}

async fn handle_resources_read(
    sessions: &SessionManager,
    headers: &HeaderMap,
    request: JsonRpcRequest,
) -> Response {
    let session_id = match extract_session_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match sessions.get(&session_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let params: ReadResourceRequestParams = match request.params.clone() {
        Some(v) => match serde_json::from_value(v) {
            Ok(p) => p,
            Err(e) => {
                return invalid_params_response(
                    request.id,
                    format!("resources/read params: {e}"),
                );
            }
        },
        None => return invalid_params_response(request.id, "missing params".into()),
    };

    let token = session.register_in_flight(&request.id);
    let _guard = InFlightGuard {
        session: Arc::clone(&session),
        id: request.id.clone(),
    };

    let result = tokio::select! {
        biased;
        _ = token.cancelled() => {
            return cancelled_response(request.id);
        }
        result = session.read_resource(&params.uri) => result,
    };

    match result {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id: request.id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(ReadResourceError::ResourceNotFound(uri)) => {
            invalid_params_response(request.id, format!("resource not found: {uri}"))
        }
        Err(ReadResourceError::Upstream(e)) => {
            internal_error_response(request.id, format!("upstream read_resource: {e}"))
        }
    }
}

// ---- Helpers --------------------------------------------------------------

fn extract_session_id(headers: &HeaderMap) -> Result<String, Response> {
    match headers.get(SESSION_ID_HEADER) {
        Some(v) => match v.to_str() {
            Ok(s) => Ok(s.to_string()),
            // Per spec, missing / unparseable session id maps to HTTP 404
            // — this isn't a JSON-RPC envelope concern, it's transport.
            Err(_) => Err((
                StatusCode::NOT_FOUND,
                format!("{SESSION_ID_HEADER} is not valid UTF-8"),
            )
                .into_response()),
        },
        None => Err((
            StatusCode::NOT_FOUND,
            format!("missing {SESSION_ID_HEADER} header"),
        )
            .into_response()),
    }
}

/// Spec-compliant 404 for "the session id was present but unknown."
/// Same shape the MCP spec mandates for session expiration.
fn unknown_session_response() -> Response {
    (StatusCode::NOT_FOUND, "unknown session").into_response()
}

/// Build a JSON-RPC `-32800 Request cancelled` error response, returned
/// when an in-flight call is cancelled via `notifications/cancelled`.
fn cancelled_response(id: serde_json::Value) -> Response {
    json_rpc_error_response(StatusCode::OK, id, REQUEST_CANCELLED, "request cancelled".into())
}

/// RAII guard that removes the in-flight cancellation token when the
/// handler future returns or is dropped (cancellation, panic, etc.).
/// Owns its `id` clone so the handler can still move `request.id` into
/// the response builders without borrow-conflicts.
struct InFlightGuard {
    session: Arc<Session>,
    id: serde_json::Value,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        self.session.deregister_in_flight(&self.id);
    }
}

/// Per Streamable HTTP spec, POST clients must declare both
/// `application/json` and `text/event-stream` (or `*/*`) in `Accept`.
/// Reject with `406 Not Acceptable` otherwise.
fn require_streamable_http_accept(headers: &HeaderMap) -> Result<(), Response> {
    let raw = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let mut json = false;
    let mut sse = false;
    let mut wildcard = false;
    for part in raw.split(',') {
        // Strip parameters like ";q=0.5" and lowercase the media type.
        let media = part.split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        match media.as_str() {
            "application/json" => json = true,
            "text/event-stream" => sse = true,
            "*/*" | "application/*" | "text/*" => wildcard = true,
            _ => {}
        }
    }

    if (json && sse) || wildcard {
        Ok(())
    } else {
        Err((
            StatusCode::NOT_ACCEPTABLE,
            "Accept header must list both application/json and text/event-stream",
        )
            .into_response())
    }
}

fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        experimental: None,
        logging: None,
        completions: None,
        prompts: None,
        // Tools and resources are exactly what `objectiveai_sdk::mcp::Connection`
        // exercises today. list_changed=true is honest: upstream
        // notifications/{tools,resources}/list_changed are forwarded onto
        // this session's SSE GET stream via Session's per-upstream
        // callbacks (see session::Session::new).
        tools: Some(ToolsCapability {
            list_changed: Some(true),
        }),
        resources: Some(ResourcesCapability {
            subscribe: None,
            list_changed: Some(true),
        }),
        tasks: None,
    }
}

fn server_info() -> Implementation {
    Implementation {
        name: "oaip".into(),
        title: None,
        version: env!("CARGO_PKG_VERSION").into(),
        website_url: None,
        description: None,
        icons: None,
    }
}

fn json_rpc_error_response(
    status: StatusCode,
    id: serde_json::Value,
    code: i64,
    message: String,
) -> Response {
    let body: JsonRpcResponse<()> = JsonRpcResponse::Error {
        jsonrpc: "2.0".into(),
        id,
        error: JsonRpcError {
            code,
            message,
            data: None,
        },
    };
    (status, Json(body)).into_response()
}


fn parse_error_response(message: String) -> Response {
    json_rpc_error_response(
        StatusCode::BAD_REQUEST,
        serde_json::Value::Null,
        PARSE_ERROR,
        message,
    )
}

fn invalid_request_response(id: serde_json::Value, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, id, INVALID_REQUEST, message)
}

fn invalid_params_response(id: serde_json::Value, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, id, INVALID_PARAMS, message)
}

fn internal_error_response(id: serde_json::Value, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, id, INTERNAL_ERROR, message)
}

fn method_not_found_response(id: serde_json::Value, method: &str) -> Response {
    json_rpc_error_response(
        StatusCode::OK,
        id,
        METHOD_NOT_FOUND,
        format!("method not found: {method}"),
    )
}

