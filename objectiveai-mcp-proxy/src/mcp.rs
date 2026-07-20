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
    CancelledNotificationParams, ClientRequestMethod, EmptyObject,
    InitializeRequestParams, JsonRpcClientMessage, JsonRpcClientNotification,
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, RequestId,
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
/// `initialize`. The proxy mints a plain random UUID for this on
/// `initialize` purely for MCP-spec compliance — it never routes on it
/// (see [`RESPONSE_ID_HEADER`]).
const SESSION_ID_HEADER: &str = "Mcp-Session-Id";

/// The header the proxy actually keys sessions on. Every objectiveai MCP
/// client (the API's own client, and the agent subprocess clients it
/// configures) re-stamps the response id on every request, so the proxy
/// resolves the owning [`Session`] from this header on every endpoint —
/// `initialize` included — and ignores the inbound `Mcp-Session-Id`.
const RESPONSE_ID_HEADER: &str = "X-OBJECTIVEAI-RESPONSE-ID";

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
    if let Err(resp) = require_streamable_http_accept(&headers) {
        return resp;
    }

    // ONE typed parse for the whole inbound message — the JSON-RPC kind
    // discrimination ("does the frame carry an id") lives inside
    // `JsonRpcClientMessage`'s deserializer, and the method → typed
    // params lowering inside `ClientRequest` / `JsonRpcClientNotification`.
    // Malformed JSON or a broken envelope returns a JSON-RPC -32700
    // rather than axum's plain-text 400. Known methods with unusable
    // params surface as `InvalidParams` (→ -32602 WITH the request id)
    // and unknown methods as `Unknown` (→ -32601), so no valid frame
    // ever loses its id to a parse failure.
    let message: JsonRpcClientMessage = match serde_json::from_slice(&body) {
        Ok(m) => m,
        Err(e) => {
            return parse_error_response(format!(
                "invalid JSON-RPC message: {e}"
            ));
        }
    };

    let request = match message {
        // notifications/cancelled is the one notification we actually
        // act on: look up the in-flight token for params.requestId in
        // the session (if any) and fire it.
        JsonRpcClientMessage::Notification(
            JsonRpcClientNotification::Cancelled { params, .. },
        ) => {
            handle_cancelled_notification(&state, &headers, &params);
            return StatusCode::ACCEPTED.into_response();
        }
        // Every other notification (initialized, unknown methods,
        // unusable cancelled params) gets the spec-mandated 202.
        JsonRpcClientMessage::Notification(_) => {
            return StatusCode::ACCEPTED.into_response();
        }
        JsonRpcClientMessage::Request(request) => request,
    };

    match request {
        JsonRpcRequest::Initialize { id, params, .. } => {
            handle_initialize(&state, &headers, id, params).await
        }
        JsonRpcRequest::Ping { id, .. } => handle_ping(id),
        // The typed params carry a pagination cursor, but the proxy's
        // aggregated lists have never consulted it — preserved as-is.
        JsonRpcRequest::ListTools { id, .. } => {
            handle_tools_list(&state.sessions, &headers, id).await
        }
        JsonRpcRequest::CallTool { id, params, .. } => {
            handle_tools_call(
                &state.sessions,
                state.queue_delegate.as_ref(),
                &headers,
                id,
                params,
            )
            .await
        }
        JsonRpcRequest::ListResources { id, .. } => {
            handle_resources_list(&state.sessions, &headers, id).await
        }
        JsonRpcRequest::ReadResource { id, params, .. } => {
            handle_resources_read(&state.sessions, &headers, id, params).await
        }
        // Fallback: the method tells invalid-params (known marker)
        // apart from method-not-found (`Other`) — see
        // `ClientRequestMethod`.
        JsonRpcRequest::Fallback { id, method, .. } => match method {
            ClientRequestMethod::Other(other) => {
                method_not_found_response(id, &other)
            }
            known => invalid_params_response(
                id,
                format!("{} params: missing or invalid", known.as_str()),
            ),
        },
    }
}

/// Look up the in-flight token for `params.requestId` and fire it. Quietly
/// no-ops if anything's missing — there's nothing to do if we can't
/// identify the request being cancelled.
fn handle_cancelled_notification(
    state: &AppState,
    headers: &HeaderMap,
    params: &CancelledNotificationParams,
) {
    let response_id = match header_response_id(headers) {
        Some(s) => s,
        None => return,
    };
    let session = match state.sessions.get(&response_id) {
        Some(s) => s,
        None => return,
    };
    let cancelled = session.cancel_in_flight(&params.request_id);
    tracing::debug!(
        response_id = %response_id,
        request_id = %params.request_id,
        cancelled,
        "notifications/cancelled received",
    );
}

// ---- DELETE handler (explicit session termination) ------------------------

/// DELETE `/`: end the session for this request's objectiveai response
/// id (`X-OBJECTIVEAI-RESPONSE-ID`). The inbound `Mcp-Session-Id` is
/// ignored, like every other non-`initialize` endpoint.
///
/// Per 2025-06-18/basic/transports#session-management the client uses
/// this to explicitly terminate its session. The proxy doesn't just
/// drop its own state — it pops the in-memory [`Session`] and fans
/// [`objectiveai_sdk::mcp::Connection::delete`] over every held
/// connection so the upstreams stop accruing per-session state on our
/// behalf. Each connection's `delete()` cancels its own listener task
/// before issuing the upstream DELETE and treats upstream
/// `404 / 401 / 403` as success.
///
/// No response id, or no session for it → `404`. (There is no longer a
/// stateless reconnect-from-id path: session ids are plain UUIDs that
/// encode nothing, so a session the proxy doesn't hold in memory can't
/// be reconstructed.)
///
/// Every per-upstream DELETE runs to completion — no short-circuit on
/// first error: `join_all` collects every result, and the handler only
/// returns an error if *any* per-upstream call failed.
pub async fn handle_delete(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    let response_id = match extract_response_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.remove(&response_id) {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let results = futures::future::join_all(
        session.connections.values().map(|c| c.delete()),
    )
    .await;
    finalize_delete_results(&results)
}

/// Reduce a `Vec<Result<(), E>>` collected from a concurrent DELETE
/// fan-out into a single HTTP response. `200` iff every per-upstream
/// delete succeeded; `500` with a body listing every failure if any
/// failed. Every call has already been awaited to completion before
/// this runs — we never short-circuit on the first error.
fn finalize_delete_results<E: std::fmt::Display>(
    results: &[Result<(), E>],
) -> Response {
    let failures: Vec<String> = results
        .iter()
        .filter_map(|r| r.as_ref().err())
        .map(|e| e.to_string())
        .collect();
    if failures.is_empty() {
        StatusCode::OK.into_response()
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "delete failed for some upstreams:\n{}",
                failures.join("\n"),
            ),
        )
            .into_response()
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
    let response_id = match extract_response_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    let session = match state.sessions.get(&response_id) {
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
    id: RequestId,
    params: InitializeRequestParams,
) -> Response {
    // Validate the client's requested protocolVersion. We don't care
    // about anything else in the params (clientInfo / capabilities) —
    // they don't change our routing or our advertised feature set.
    // (A missing/non-string protocolVersion never reaches here: it
    // lands on `ClientRequest::InvalidParams` in the dispatcher →
    // -32602, same as before.)
    if !ACCEPTED_PROTOCOL_VERSIONS
        .contains(&params.protocol_version.as_str())
    {
        // Accepted versions downgrade in the response to
        // PROTOCOL_VERSION and a spec-compliant client adopts it.
        return invalid_request_response(
            id,
            format!(
                "unsupported protocolVersion {:?}; this proxy accepts {ACCEPTED_PROTOCOL_VERSIONS:?}",
                params.protocol_version,
            ),
        );
    }

    // Routing keys off the objectiveai response id, NOT the inbound
    // `Mcp-Session-Id` — the latter is ignored for branch selection here
    // (and everywhere else). Every objectiveai MCP client sends the
    // response id on every request; a fresh connect and a reconnect look
    // identical to the proxy and resolve the same way. Two outcomes:
    //
    //   1. A session for this response id is already live in memory →
    //      REUSE it. The new request's `X-MCP-Servers` / `X-MCP-Headers`
    //      are NOT re-dialed; we just refresh the session-global
    //      transient headers from this request's HeaderMap.
    //   2. No session yet → FRESH CONNECT. `X-MCP-Servers` /
    //      `X-MCP-Headers` build the spec list, every URL connects from
    //      scratch, and the opened upstreams are registered under the
    //      response id.
    //
    // Both outcomes return a fresh random-UUID `Mcp-Session-Id` purely
    // for MCP-spec compliance; the proxy never routes on it.
    let response_id = match extract_response_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // A dropped (banned) response id is never (re)admitted — skip the
    // reuse/connect entirely. A banned id's session was already torn
    // down, so the reuse path below wouldn't find it anyway; this also
    // avoids a wasted upstream connect.
    if state.dropper.as_ref().is_some_and(|d| d.is_banned(&response_id)) {
        return invalid_request_response(id, "response id has been dropped".to_string());
    }

    let mcp_session_id = uuid::Uuid::new_v4().to_string();

    // Double-initialize hardening: if another initialize for this id is
    // mid-connect, wait it out instead of dialing every upstream a
    // second time. A session appearing means the concurrent initialize
    // won — fall through to the reuse path below.
    if state.sessions.initializing(&response_id).is_some() {
        let _ = state.sessions.get_or_wait(&response_id).await;
    }

    if let Some(session) = state.sessions.get(&response_id) {
        // Outcome 1 — reuse the live in-memory session. Re-apply the
        // session-global transient headers from THIS request's inbound
        // HeaderMap (full replace — missing keys drop from the bag).
        session.apply_transient_headers(headers).await;
        return ok_response_resume_sse(id, mcp_session_id);
    }

    // Mark the fresh connect in flight: client requests for this id
    // (`tools/list` & co — notably from an upstream server calling back
    // in while it is itself being connected) park on this marker via
    // `get_or_wait` instead of 404ing. The guard's Drop releases them
    // on EVERY exit path below, so no request waits past this
    // function's return.
    let _init_guard = state.sessions.begin_initializing(&response_id);

    // Outcome 2 — fresh connect. `X-MCP-Servers` / `X-MCP-Headers` build
    // the spec list, every URL connects from scratch, and the opened
    // upstreams are registered under the response id. The agent-identity
    // and response-routing headers ride on `Session::transient_headers`
    // (applied below).
    let (connections, transient_headers) = match crate::upstream::connect_all_fresh(
        &state.client,
        state.reverse_channel.as_ref(),
        headers,
    ).await {
        Ok(conns) => conns,
        Err(e @ (BadInit::NotUtf8 { .. } | BadInit::NotJson { .. })) => {
            return invalid_request_response(id, e.to_string());
        }
        Err(e @ BadInit::UpstreamConnectFailed { .. }) => {
            return internal_error_response(id, e.to_string());
        }
        Err(e @ BadInit::UpstreamListFailed { .. }) => {
            // A post-connect tools/resources probe failed: the
            // upstream accepted initialize but can't serve. Same
            // outcome as a connect failure — the session is not viable.
            return internal_error_response(id, e.to_string());
        }
    };
    state
        .sessions
        .add(response_id.clone(), connections, transient_headers);
    // Race guard: a `drop` may have banned this id while we were
    // connecting. We ban-then-check on the drop side and insert-then-
    // check here, so one side always tears the session down. Teardown is
    // idempotent.
    if state.dropper.as_ref().is_some_and(|d| d.is_banned(&response_id)) {
        crate::dropper::teardown(&response_id, &state.sessions, state.reverse_channel.as_ref()).await;
        return invalid_request_response(id, "response id has been dropped".to_string());
    }
    // Stamp the session-global transient headers extracted from the
    // inbound HeaderMap.
    if let Some(session) = state.sessions.get(&response_id) {
        session.apply_transient_headers(headers).await;
    }
    ok_response_fresh_sse(id, mcp_session_id)
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
    request_id: RequestId,
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
        id: request_id.clone(),
        result,
    };
    let payload = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(e) => {
            return internal_error_response(
                request_id,
                format!("failed to serialize InitializeResult: {e}"),
            );
        }
    };

    let header_value = match HeaderValue::from_str(&session_id) {
        Ok(v) => v,
        Err(_) => {
            return internal_error_response(
                request_id,
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

/// Reuse-init `initialize` response: a session for this response id was
/// already live in memory, so no upstreams were re-dialed. Matches
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
/// Echoes a fresh random-UUID `Mcp-Session-Id` response header
/// (`session_id`). The proxy doesn't route on it — every subsequent
/// request resolves by `X-OBJECTIVEAI-RESPONSE-ID` — but a valid
/// `Mcp-Session-Id` is returned for MCP-spec compliance so 3rd-party
/// clients (e.g. `claude_agent_sdk`'s bundled CLI) accept the response.
fn ok_response_resume_sse(
    request_id: RequestId,
    session_id: String,
) -> Response {
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
        id: request_id.clone(),
        result,
    };
    let payload = match serde_json::to_string(&body) {
        Ok(s) => s,
        Err(e) => {
            return internal_error_response(
                request_id,
                format!("failed to serialize InitializeResult: {e}"),
            );
        }
    };
    let result_event = sse_stream::Sse::default().data(payload);

    let header_value = match HeaderValue::from_str(&session_id) {
        Ok(v) => v,
        Err(_) => {
            return internal_error_response(
                request_id,
                format!("session id is not a valid header value: {session_id}"),
            );
        }
    };

    let stream = stream::iter(vec![
        Ok::<sse_stream::Sse, Infallible>(priming),
        Ok(result_event),
    ])
    .chain(stream::pending::<Result<sse_stream::Sse, Infallible>>());
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

fn handle_ping(id: RequestId) -> Response {
    let body: JsonRpcResponse<EmptyObject> = JsonRpcResponse::Success {
        jsonrpc: "2.0".into(),
        id,
        result: EmptyObject {},
    };
    (StatusCode::OK, Json(body)).into_response()
}

async fn handle_tools_list(
    sessions: &SessionManager,
    headers: &HeaderMap,
    id: RequestId,
) -> Response {
    let response_id = match extract_response_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // A mid-initial-connect id parks here until the connect finishes.
    let session = match sessions.get_or_wait(&response_id).await {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    // Optional `X-List-Filter`: scope the fan-out to a single
    // upstream URL. Absent → fan out to every upstream. Same header
    // applies to both `tools/list` and `resources/list`.
    let filter_url = headers
        .get(crate::upstream::LIST_FILTER_HEADER)
        .and_then(|v| v.to_str().ok());

    match session.list_tools_filtered(filter_url, None).await {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => internal_error_response(id, format!("list_tools: {e}")),
    }
}

async fn handle_tools_call(
    sessions: &SessionManager,
    queue_delegate: Option<&Arc<dyn crate::QueueDelegate>>,
    headers: &HeaderMap,
    id: RequestId,
    params: CallToolRequestParams,
) -> Response {
    let response_id = match extract_response_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // A mid-initial-connect id parks here until the connect finishes.
    let session = match sessions.get_or_wait(&response_id).await {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let token = session.register_in_flight(&id);
    let _guard = InFlightGuard {
        session: Arc::clone(&session),
        id: id.clone(),
    };

    let tool_result = tokio::select! {
        biased;
        _ = token.cancelled() => {
            return cancelled_response(id);
        }
        result = session.call_tool(&params) => result,
    };

    match tool_result {
        Ok(mut result) => {
            // Tool call succeeded — only NOW do we read the queue.
            // Running the read in parallel with the tool call
            // would advance the delegate's ban list before we
            // know whether the proxy is going to return a JSON-RPC
            // error (ToolNotFound / Upstream), in which case the
            // consumed rows would never reach the agent. Reading
            // sequentially after success guarantees every consumed
            // row gets surfaced.
            let agent_arguments = session.transient_headers.read().await.clone();
            if let Some(crate::QueueRead { token, blocks }) =
                maybe_read_blocks(queue_delegate, &agent_arguments).await
            {
                // Splice the queued rows ahead of the upstream's
                // tool-result content, wrapped in the SDK-owned
                // `<system-reminder>` text-block pair whose prefix
                // carries the confirmation token. The API's run-loop
                // regex-scans tool message text for this token and calls
                // back via `confirm()` to finalize delivery — closing the
                // "claimed delivered but never reached the agent" hole a
                // naive ban list would leave open. Each queued row's
                // content is separated by a `\n\n` text part so distinct
                // messages stay demarcated instead of running together.
                // `count` (surfaced as `_meta.notifications`) counts the
                // real content blocks, not the separators.
                let mut pending: Vec<ContentBlock> = Vec::new();
                let mut count: u64 = 0;
                for row in blocks {
                    if row.is_empty() {
                        continue;
                    }
                    if !pending.is_empty() {
                        pending.push(ContentBlock::Text(TextContent {
                            text: "\n\n".to_string(),
                            annotations: None,
                            _meta: None,
                        }));
                    }
                    count += row.len() as u64;
                    pending.extend(row);
                }
                if !pending.is_empty() {
                    let mut prefixed = Vec::with_capacity(
                        2 + pending.len() + result.content.len(),
                    );
                    prefixed.push(ContentBlock::Text(TextContent {
                        text: objectiveai_sdk::mcp::queue_notification::format_prefix(&token),
                        annotations: None,
                        _meta: None,
                    }));
                    prefixed.extend(pending);
                    prefixed.push(ContentBlock::Text(TextContent {
                        text: objectiveai_sdk::mcp::queue_notification::format_suffix(&token),
                        annotations: None,
                        _meta: None,
                    }));
                    prefixed.append(&mut result.content);
                    result.content = prefixed;
                    // Surface the count as `_meta.notifications` so
                    // downstream consumers (SDK's
                    // `call_tool_as_message`) can read it
                    // structurally without parsing content blocks.
                    let meta = result._meta.get_or_insert_with(indexmap::IndexMap::new);
                    meta.insert(
                        "notifications".to_string(),
                        serde_json::Value::Number(count.into()),
                    );
                }
            }
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(CallToolError::ToolNotFound(name)) => {
            method_not_found_response(id, &format!("tool: {name}"))
        }
        Err(CallToolError::Upstream(e)) => {
            internal_error_response(id, format!("upstream call_tool: {e}"))
        }
    }
}

/// Short-circuit wrapper: returns `None` when no delegate is
/// installed, otherwise forwards to the trait method.
async fn maybe_read_blocks(
    delegate: Option<&Arc<dyn crate::QueueDelegate>>,
    agent_arguments: &indexmap::IndexMap<String, String>,
) -> Option<crate::QueueRead> {
    delegate?.read_pending_blocks(agent_arguments).await
}

async fn handle_resources_list(
    sessions: &SessionManager,
    headers: &HeaderMap,
    id: RequestId,
) -> Response {
    let response_id = match extract_response_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // A mid-initial-connect id parks here until the connect finishes.
    let session = match sessions.get_or_wait(&response_id).await {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    // Optional `X-List-Filter`: scope the fan-out to a single
    // upstream URL. Absent → fan out to every upstream. Same header
    // semantics as `tools/list`.
    let filter_url = headers
        .get(crate::upstream::LIST_FILTER_HEADER)
        .and_then(|v| v.to_str().ok());

    match session.list_resources_filtered(filter_url, None).await {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(e) => internal_error_response(id, format!("list_resources: {e}")),
    }
}

async fn handle_resources_read(
    sessions: &SessionManager,
    headers: &HeaderMap,
    id: RequestId,
    params: ReadResourceRequestParams,
) -> Response {
    let response_id = match extract_response_id(headers) {
        Ok(id) => id,
        Err(resp) => return resp,
    };

    // A mid-initial-connect id parks here until the connect finishes.
    let session = match sessions.get_or_wait(&response_id).await {
        Some(s) => s,
        None => return unknown_session_response(),
    };

    let token = session.register_in_flight(&id);
    let _guard = InFlightGuard {
        session: Arc::clone(&session),
        id: id.clone(),
    };

    let result = tokio::select! {
        biased;
        _ = token.cancelled() => {
            return cancelled_response(id);
        }
        result = session.read_resource(&params.uri) => result,
    };

    match result {
        Ok(result) => {
            let body = JsonRpcResponse::Success {
                jsonrpc: "2.0".into(),
                id,
                result,
            };
            (StatusCode::OK, Json(body)).into_response()
        }
        Err(ReadResourceError::ResourceNotFound(uri)) => {
            invalid_params_response(id, format!("resource not found: {uri}"))
        }
        Err(ReadResourceError::Upstream(e)) => {
            internal_error_response(id, format!("upstream read_resource: {e}"))
        }
    }
}

// ---- Helpers --------------------------------------------------------------

/// Read the objectiveai response id from [`RESPONSE_ID_HEADER`]. This is
/// the key the proxy routes every endpoint on. Unlike `Mcp-Session-Id`,
/// it is never comma-joined by SDK transports (it's a custom header the
/// objectiveai clients set explicitly), so a single verbatim read.
fn header_response_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(RESPONSE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
}

/// Resolve the routing key for any request: the objectiveai response id.
/// Absent → 404 (transport-level — the same shape the proxy used for a
/// missing `Mcp-Session-Id` before it switched to response-id routing).
fn extract_response_id(headers: &HeaderMap) -> Result<String, Response> {
    match header_response_id(headers) {
        Some(id) => Ok(id),
        None => Err((
            StatusCode::NOT_FOUND,
            format!("missing {RESPONSE_ID_HEADER} header"),
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
fn cancelled_response(id: RequestId) -> Response {
    json_rpc_error_response(StatusCode::OK, Some(id), REQUEST_CANCELLED, "request cancelled".into())
}

/// RAII guard that removes the in-flight cancellation token when the
/// handler future returns or is dropped (cancellation, panic, etc.).
/// Owns its `id` clone so the handler can still move `id` into
/// the response builders without borrow-conflicts.
struct InFlightGuard {
    session: Arc<Session>,
    id: RequestId,
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

/// `id: None` serializes as an explicit `"id": null` — the JSON-RPC
/// parse-error shape (no identifiable request to correlate to).
fn json_rpc_error_response(
    status: StatusCode,
    id: Option<RequestId>,
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
    json_rpc_error_response(StatusCode::BAD_REQUEST, None, PARSE_ERROR, message)
}

fn invalid_request_response(id: RequestId, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, Some(id), INVALID_REQUEST, message)
}

fn invalid_params_response(id: RequestId, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, Some(id), INVALID_PARAMS, message)
}

fn internal_error_response(id: RequestId, message: String) -> Response {
    json_rpc_error_response(StatusCode::OK, Some(id), INTERNAL_ERROR, message)
}

fn method_not_found_response(id: RequestId, method: &str) -> Response {
    json_rpc_error_response(
        StatusCode::OK,
        Some(id),
        METHOD_NOT_FOUND,
        format!("method not found: {method}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hm(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(RESPONSE_ID_HEADER, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn response_id_read_verbatim() {
        // The response id is a custom header objectiveai clients set
        // explicitly; it's read verbatim (no comma-join normalization).
        assert_eq!(header_response_id(&hm("resp-abc123")), Some("resp-abc123".to_string()));
    }

    #[test]
    fn response_id_missing_is_none() {
        assert_eq!(header_response_id(&HeaderMap::new()), None);
    }

    #[test]
    fn extract_response_id_errors_when_absent() {
        // Missing header → an error Response (the 404 is asserted at the
        // integration level; here we only need the Err arm).
        assert!(extract_response_id(&HeaderMap::new()).is_err());
        assert!(extract_response_id(&hm("resp-1")).is_ok());
    }
}

