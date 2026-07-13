//! WebSocket variants of the 8 streaming endpoints (stage 1 of #193).
//!
//! Each `_ws` handler:
//!
//! 1. Reads the request body as a single text frame on the full socket
//!    (`streaming_ws::recv_body_frame`).
//! 2. Calls `create_streaming_*` to set up the chunk stream.
//! 3. Splits the socket into a `SharedSink` (mutex-wrapped sender) and
//!    a `SplitStream` (receiver).
//! 4. Runs two concurrent futures under `tokio::select!`:
//!    - **send**: drains the chunk stream through a one-ahead buffer,
//!      forwarding each chunk as a JSON text frame while HOLDING the
//!      newest one back. At end of stream the held FINAL chunk is
//!      returned to the select arm, which drains every in-flight
//!      client_request handler first, then emits the final chunk and
//!      `Close(1000)` back-to-back — last chunk = connection end.
//!    - **recv**: parses incoming text frames as
//!      [`client_request::Request`](objectiveai_sdk::client_objectiveai_mcp::client_request::Request)
//!      or [`server_response::Response`](objectiveai_sdk::client_objectiveai_mcp::server_response::Response)
//!      and demultiplexes them (`streaming_ws::recv_loop`): MCP traffic
//!      to the per-request proxy's reverse channel, message-queue
//!      responses to the API's pending-request registry. Sends back the
//!      matching
//!      [`client_response::Response`](objectiveai_sdk::client_objectiveai_mcp::client_response::Response)
//!      for every parsed client request.
//!
//! Errors during setup land as `Close(1011)` after a `ResponseError`
//! text frame; body-deserialize failures land as `Close(1003)`.
//!
//! The `stream` field on the request body is ignored on this path —
//! opening a WS implies streaming intent.

use axum::extract::ws::{WebSocketUpgrade, close_code};
use futures::StreamExt as _;
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

use crate::{
    agent, ctx, functions, retrieval, streaming_ws, vector,
};
use crate::functions::profiles::computations::Client as _;

pub(crate) async fn create_agent_completion_ws(
    client: Arc<
        agent::completions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
            > + Send + Sync + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
            > + Send + Sync + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
            > + Send + Sync + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
            > + Send + Sync + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
            > + Send + Sync + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    suppress_output: bool,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        // Build the reverse-attach plumbing BEFORE the stream so the
        // agent client can register per-agent `response_id`s against
        // this WS's reverse channel from inside its swarm-iteration
        // site (synthesized `/objectiveai` and `/{owner}/{name}/{ver}/{mcp}`
        // URLs both look up by `X-OBJECTIVEAI-RESPONSE-ID`). The guard
        // is held for the entire `on_upgrade` async block — when it
        // drops, all registered ids are removed.
        let pending = streaming_ws::new_pending_requests();
        // Tracks in-flight client_request handlers spawned by `recv_loop`
        // so we can keep the WS open until they've written their replies.
        let tasks = tokio_util::task::TaskTracker::new();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            sink.clone(),
            pending.clone(),
        );
        // No channel-level budget: every reverse-channel op passes its
        // own per-op timeout (ws-MCP calls use the request's
        // `X-MCP-CALL-TIMEOUT`; connects use the connect timeout;
        // laboratory transfers and the API's OWN server_requests wait
        // forever on the CLI).
        let (reverse_channel, reverse_req_rx) =
            objectiveai_mcp_proxy::ReverseChannel::new();
        tokio::spawn(streaming_ws::drain_reverse_channel(sink.clone(), reverse_req_rx));
        let ctx = crate::context(&headers, suppress_output)
            .with_reverse_attach(_attach_guard.handle())
            .with_reverse_channel(reverse_channel.clone());

        // Stream setup lives INSIDE the `send` branch so the `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`'s
        // first-chunk await. See `create_vector_completion_ws` for the
        // full rationale — same deadlock pattern, same fix.
        let send_sink = sink.clone();        let send = async move {
            let stream = match client
                .create_streaming_handle_usage(
                    ctx,
                    Arc::new(body),
                    None,
                    None,
                    vec![],
                    indexmap::IndexMap::new(),
                    None,
                )
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return None;
                }
            };
            let mut stream = Box::pin(stream);
            // One-ahead buffer: hold the newest frame, send its
            // predecessor. The frame still held at end-of-stream is the
            // FINAL chunk — returned (not sent) so the select arm can
            // drain in-flight MCP stragglers before emitting it. Last
            // chunk = connection end.
            let mut held: Option<String> = None;
            while let Some(item) = stream.next().await {
                let agent::completions::StreamItem::Chunk(chunk) = item else { continue };
                let frame = match serde_json::to_string(&chunk) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if let Some(prev) = held.replace(frame) {
                    if streaming_ws::send_frame_split(&send_sink, prev).await.is_err() {
                        return None;
                    }
                }
            }
            held
        };

        let recv = streaming_ws::recv_loop(
            rx,
            sink.clone(),
            pending,
            reverse_channel,
            tasks.clone(),
        );

        tokio::select! {
            // Send won: agent stream done, its FINAL chunk still in hand.
            // Drain any in-flight client_request handlers (write their
            // replies), THEN emit the final chunk + Close — last chunk =
            // connection end.
            final_frame = send => {
                streaming_ws::drain_send_final_and_close(tasks, &sink, final_frame).await
            }
            // Recv won: peer closed / recv error — nothing to deliver.
            _ = recv => {}
        }
    })
}

pub(crate) async fn create_vector_completion_ws<
    OR, CAG, CX, MK, SPT, RG, RF, RM, AU, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        vector::completions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, SPT, RG, RF, RM, AU,
            impl vector::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
        >,
    >,
    _agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>,
    >,
    headers: axum::http::HeaderMap,
    suppress_output: bool,
    ws: WebSocketUpgrade,
) -> axum::response::Response
where
    OR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    CAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    CX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    MK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    SPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    NOR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    NCAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    NCX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    NMK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    NSPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let pending = streaming_ws::new_pending_requests();
        // Tracks in-flight client_request handlers spawned by `recv_loop`
        // so we can keep the WS open until they've written their replies.
        let tasks = tokio_util::task::TaskTracker::new();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            sink.clone(),
            pending.clone(),
        );
        // No channel-level budget: every reverse-channel op passes its
        // own per-op timeout (ws-MCP calls use the request's
        // `X-MCP-CALL-TIMEOUT`; connects use the connect timeout;
        // laboratory transfers and the API's OWN server_requests wait
        // forever on the CLI).
        let (reverse_channel, reverse_req_rx) =
            objectiveai_mcp_proxy::ReverseChannel::new();
        tokio::spawn(streaming_ws::drain_reverse_channel(sink.clone(), reverse_req_rx));
        let ctx = crate::context(&headers, suppress_output)
            .with_reverse_attach(_attach_guard.handle())
            .with_reverse_channel(reverse_channel.clone());

        // `create_streaming_handle_usage` lives INSIDE the `send`
        // branch so the `recv_loop` (which dispatches incoming
        // `server_response` frames to the MCP-endpoint pending
        // oneshots) is polled concurrently with stream setup. If we
        // awaited stream creation OUTSIDE the select, agents would
        // deadlock waiting on responses the recv_loop wasn't draining
        // yet — see the 60s WS-cascade bug fix.
        let send_sink = sink.clone();        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return None;
                }
            };
            let mut stream = Box::pin(stream);
            // One-ahead buffer — the held frame at end-of-stream is the
            // FINAL chunk, withheld for the drain (see the agent handler).
            let mut held: Option<String> = None;
            while let Some(chunk) = stream.next().await {
                let frame = match serde_json::to_string(&chunk) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if let Some(prev) = held.replace(frame) {
                    if streaming_ws::send_frame_split(&send_sink, prev).await.is_err() {
                        return None;
                    }
                }
            }
            held
        };

        let recv = streaming_ws::recv_loop(
            rx,
            sink.clone(),
            pending,
            reverse_channel,
            tasks.clone(),
        );

        tokio::select! {
            // Send won: stream done, FINAL chunk in hand. Drain in-flight
            // client_request handlers, then emit the final chunk + Close.
            final_frame = send => {
                streaming_ws::drain_send_final_and_close(tasks, &sink, final_frame).await
            }
            // Recv won: peer closed / recv error — nothing to deliver.
            _ = recv => {}
        }
    })
}

pub(crate) async fn execute_function_ws<
    OR, CAG, CX, MK, SPT, AU, VAU, RG, RF, RM, FAU, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        functions::executions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, SPT, AU, VAU, RG, RF, RM, FAU,
        >,
    >,
    _agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>,
    >,
    headers: axum::http::HeaderMap,
    suppress_output: bool,
    ws: WebSocketUpgrade,
) -> axum::response::Response
where
    OR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    CAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    CX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    MK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    SPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    VAU: vector::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    FAU: functions::executions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    NOR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    NCAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    NCX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    NMK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    NSPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let pending = streaming_ws::new_pending_requests();
        // Tracks in-flight client_request handlers spawned by `recv_loop`
        // so we can keep the WS open until they've written their replies.
        let tasks = tokio_util::task::TaskTracker::new();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            sink.clone(),
            pending.clone(),
        );
        // No channel-level budget: every reverse-channel op passes its
        // own per-op timeout (ws-MCP calls use the request's
        // `X-MCP-CALL-TIMEOUT`; connects use the connect timeout;
        // laboratory transfers and the API's OWN server_requests wait
        // forever on the CLI).
        let (reverse_channel, reverse_req_rx) =
            objectiveai_mcp_proxy::ReverseChannel::new();
        tokio::spawn(streaming_ws::drain_reverse_channel(sink.clone(), reverse_req_rx));
        let ctx = crate::context(&headers, suppress_output)
            .with_reverse_attach(_attach_guard.handle())
            .with_reverse_channel(reverse_channel.clone());

        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`.
        // See `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return None;
                }
            };
            let mut stream = Box::pin(stream);
            // One-ahead buffer — the held frame at end-of-stream is the
            // FINAL chunk, withheld for the drain (see the agent handler).
            let mut held: Option<String> = None;
            while let Some(chunk) = stream.next().await {
                let frame = match serde_json::to_string(&chunk) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                if let Some(prev) = held.replace(frame) {
                    if streaming_ws::send_frame_split(&send_sink, prev).await.is_err() {
                        return None;
                    }
                }
            }
            held
        };

        let recv = streaming_ws::recv_loop(
            rx,
            sink.clone(),
            pending,
            reverse_channel,
            tasks.clone(),
        );

        tokio::select! {
            // Send won: stream done, FINAL chunk in hand. Drain in-flight
            // client_request handlers, then emit the final chunk + Close.
            final_frame = send => {
                streaming_ws::drain_send_final_and_close(tasks, &sink, final_frame).await
            }
            // Recv won: peer closed / recv error — nothing to deliver.
            _ = recv => {}
        }
    })
}

pub(crate) async fn create_profile_computation_ws<NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>(
    client: Arc<functions::profiles::computations::ObjectiveAiClient>,
    _agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>,
    >,
    headers: axum::http::HeaderMap,
    suppress_output: bool,
    ws: WebSocketUpgrade,
) -> axum::response::Response
where
    NOR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    NCAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    NCX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    NMK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    NSPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::functions::profiles::computations::request::FunctionProfileComputationCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let pending = streaming_ws::new_pending_requests();
        // Tracks in-flight client_request handlers spawned by `recv_loop`
        // so we can keep the WS open until they've written their replies.
        let tasks = tokio_util::task::TaskTracker::new();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            sink.clone(),
            pending.clone(),
        );
        // No channel-level budget: every reverse-channel op passes its
        // own per-op timeout (ws-MCP calls use the request's
        // `X-MCP-CALL-TIMEOUT`; connects use the connect timeout;
        // laboratory transfers and the API's OWN server_requests wait
        // forever on the CLI).
        let (reverse_channel, reverse_req_rx) =
            objectiveai_mcp_proxy::ReverseChannel::new();
        tokio::spawn(streaming_ws::drain_reverse_channel(sink.clone(), reverse_req_rx));
        let ctx = crate::context(&headers, suppress_output)
            .with_reverse_attach(_attach_guard.handle())
            .with_reverse_channel(reverse_channel.clone());

        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming`. See
        // `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();        let send = async move {
            let stream = match client.create_streaming(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(&send_sink, &e).await;
                    return None;
                }
            };
            let mut stream = Box::pin(stream);
            // One-ahead buffer — the held frame at end-of-stream (chunk
            // or error alike) is the FINAL frame, withheld for the drain
            // (see the agent handler).
            let mut held: Option<String> = None;
            while let Some(item) = stream.next().await {
                let frame = match &item {
                    Ok(chunk) => match serde_json::to_string(chunk) {
                        Ok(s) => s,
                        Err(_) => continue,
                    },
                    Err(err) => match serde_json::to_string(err) {
                        Ok(s) => s,
                        Err(_) => continue,
                    },
                };
                if let Some(prev) = held.replace(frame) {
                    if streaming_ws::send_frame_split(&send_sink, prev).await.is_err() {
                        return None;
                    }
                }
            }
            held
        };

        let recv = streaming_ws::recv_loop(
            rx,
            sink.clone(),
            pending,
            reverse_channel,
            tasks.clone(),
        );

        tokio::select! {
            // Send won: stream done, FINAL frame in hand. Drain in-flight
            // client_request handlers, then emit the final frame + Close.
            final_frame = send => {
                streaming_ws::drain_send_final_and_close(tasks, &sink, final_frame).await
            }
            // Recv won: peer closed / recv error — nothing to deliver.
            _ = recv => {}
        }
    })
}

pub(crate) async fn create_error_ws<NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>(
    client: Arc<crate::error::Client>,
    _agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NSPT, NRG, NRF, NRM, NAU>,
    >,
    headers: axum::http::HeaderMap,
    suppress_output: bool,
    ws: WebSocketUpgrade,
) -> axum::response::Response
where
    NOR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        > + Send + Sync + 'static,
    NCAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send + Sync + 'static,
    NCX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send + Sync + 'static,
    NMK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        > + Send + Sync + 'static,
    NSPT: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::script::Agent, objectiveai_sdk::agent::script::Continuation,
        > + Send + Sync + 'static,
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::error::request::ErrorCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let pending = streaming_ws::new_pending_requests();
        // Tracks in-flight client_request handlers spawned by `recv_loop`
        // so we can keep the WS open until they've written their replies.
        let tasks = tokio_util::task::TaskTracker::new();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            sink.clone(),
            pending.clone(),
        );
        // No channel-level budget: every reverse-channel op passes its
        // own per-op timeout (ws-MCP calls use the request's
        // `X-MCP-CALL-TIMEOUT`; connects use the connect timeout;
        // laboratory transfers and the API's OWN server_requests wait
        // forever on the CLI).
        let (reverse_channel, reverse_req_rx) =
            objectiveai_mcp_proxy::ReverseChannel::new();
        tokio::spawn(streaming_ws::drain_reverse_channel(sink.clone(), reverse_req_rx));
        let ctx = crate::context(&headers, suppress_output)
            .with_reverse_attach(_attach_guard.handle())
            .with_reverse_channel(reverse_channel.clone());
        let stream = match client.create_streaming(&ctx, &body) {
            Ok(s) => s,
            Err(e) => {
                streaming_ws::fatal_setup_error_split(&sink, &e).await;
                return;
            }
        };

        let send_sink = sink.clone();
        let send = async move {
            let mut stream = Box::pin(stream);
            // One-ahead buffer — the held frame at end-of-stream (chunk
            // or error alike) is the FINAL frame, withheld for the drain
            // (see the agent handler).
            let mut held: Option<String> = None;
            while let Some(item) = stream.next().await {
                let frame = match item {
                    Ok(chunk) => match serde_json::to_string(&chunk) {
                        Ok(s) => s,
                        Err(_) => continue,
                    },
                    Err(err) => match serde_json::to_string(&err) {
                        Ok(s) => s,
                        Err(_) => continue,
                    },
                };
                if let Some(prev) = held.replace(frame) {
                    if streaming_ws::send_frame_split(&send_sink, prev).await.is_err() {
                        return None;
                    }
                }
            }
            held
        };

        let recv = streaming_ws::recv_loop(
            rx,
            sink.clone(),
            pending,
            reverse_channel,
            tasks.clone(),
        );

        tokio::select! {
            // Send won: stream done, FINAL frame in hand. Drain in-flight
            // client_request handlers, then emit the final frame + Close.
            final_frame = send => {
                streaming_ws::drain_send_final_and_close(tasks, &sink, final_frame).await
            }
            // Recv won: peer closed / recv error — nothing to deliver.
            _ = recv => {}
        }
    })
}
