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
//!    - **send**: drains the chunk stream, observes each chunk's
//!      `agent_completion_ids` into a per-connection `SessionTracker`,
//!      and forwards the chunk as a JSON text frame. Closes 1000 at
//!      end of stream.
//!    - **recv**: parses incoming text frames as
//!      [`client_request::Request`](objectiveai_sdk::client_objectiveai_mcp::client_request::Request),
//!      validates each notify's `response_id` against the tracker, and
//!      dispatches accepted ones to `agent_completions_client.notify()`.
//!      Sends back the matching
//!      [`client_response::Response`](objectiveai_sdk::client_objectiveai_mcp::client_response::Response)
//!      for every parsed request.
//!
//! Errors during setup land as `Close(1011)` after a `ResponseError`
//! text frame; body-deserialize failures land as `Close(1003)`.
//!
//! The `stream` field on the request body is ignored on this path —
//! opening a WS implies streaming intent.

use axum::extract::ws::{WebSocketUpgrade, close_code};
use futures::{SinkExt as _, StreamExt as _};
use objectiveai_sdk::error::ResponseError;
use std::sync::Arc;

use crate::{
    agent, ctx, functions, retrieval, streaming_ws, vector,
};
use crate::functions::profiles::computations::Client as _;

/// Build the `notify_fn` closure for `streaming_ws::recv_notify_loop`
/// from an agent completions client. Captures the Arc once; each call
/// clones it again for the spawned dispatch future.
fn notify_fn_for<OR, CAG, CX, MK, RG, RF, RM, AU>(
    agent_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, OR, CAG, CX, MK, RG, RF, RM, AU>,
    >,
) -> impl Fn(
    objectiveai_sdk::agent::completions::request::AgentCompletionNotifyParams,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<(), agent::completions::Error>> + Send>,
> + Send
+ Sync
+ 'static
where
    OR: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent,
            objectiveai_sdk::agent::openrouter::Continuation,
        > + Send
        + Sync
        + 'static,
    CAG: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent,
            objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CX: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent,
            objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MK: agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent,
            objectiveai_sdk::agent::mock::Continuation,
        > + Send
        + Sync
        + 'static,
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    move |params| {
        let c = agent_client.clone();
        Box::pin(async move { c.notify(params).await })
    }
}

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
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
        >,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
        // agent client can register per-agent `ws_session_id`s
        // (synthesized for `client_objectiveai_mcp` URLs) against
        // this WS's reverse channel from inside its swarm-iteration
        // site. The guard is held for the entire `on_upgrade` async
        // block — when it drops, all registered ids are removed.
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());
        let notify_client = client.clone();

        // Stream setup lives INSIDE the `send` branch so the `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`'s
        // first-chunk await. See `create_vector_completion_ws` for the
        // full rationale — same deadlock pattern, same fix.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client
                .create_streaming_handle_usage(
                    ctx,
                    Arc::new(body),
                    None,
                    None,
                    vec![],
                    indexmap::IndexMap::new(),
                    None,
                    true,
                    None,
                    None,
                    None,
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
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(item) = stream.next().await {
                let agent::completions::StreamItem::Chunk(chunk) = item else { continue };
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(notify_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn create_vector_completion_ws<
    OR, CAG, CX, MK, RG, RF, RM, AU, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        vector::completions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, RG, RF, RM, AU,
            impl vector::completions::completion_votes_fetcher::Fetcher<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<ctx::DefaultContextExt> + Send + Sync + 'static,
            impl vector::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
        >,
    >,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());

        // `create_streaming_handle_usage` lives INSIDE the `send`
        // branch so the `recv_loop` (which dispatches incoming
        // `server_response` frames to the MCP-endpoint pending
        // oneshots) is polled concurrently with stream setup. If we
        // awaited stream creation OUTSIDE the select, agents would
        // deadlock waiting on responses the recv_loop wasn't draining
        // yet — see the 60s WS-cascade bug fix.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(chunk) = stream.next().await {
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn execute_function_ws<
    OR, CAG, CX, MK, AU, CVF, CACHEF, VAU, RG, RF, RM, FAU, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        functions::executions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, AU, CVF, CACHEF, VAU, RG, RF, RM, FAU,
        >,
    >,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    CVF: vector::completions::completion_votes_fetcher::Fetcher<ctx::DefaultContextExt> + Send + Sync + 'static,
    CACHEF: vector::completions::cache_vote_fetcher::Fetcher<ctx::DefaultContextExt> + Send + Sync + 'static,
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
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());

        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`.
        // See `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(chunk) = stream.next().await {
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn create_profile_computation_ws<NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>(
    client: Arc<functions::profiles::computations::ObjectiveAiClient>,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());

        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming`. See
        // `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client.create_streaming(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(&send_sink, &e).await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(item) = stream.next().await {
                let frame = match &item {
                    Ok(chunk) => {
                        send_tracker.observe(chunk);
                        match serde_json::to_string(chunk) {
                            Ok(s) => s,
                            Err(_) => continue,
                        }
                    }
                    Err(err) => match serde_json::to_string(err) {
                        Ok(s) => s,
                        Err(_) => continue,
                    },
                };
                let mut guard = send_sink.lock().await;
                if guard
                    .send(axum::extract::ws::Message::Text(frame.into()))
                    .await
                    .is_err()
                {
                    return;
                }
                drop(guard);
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn create_function_invention_ws<
    OR, CAG, CX, MK, RG, RF, RM, AU, IAU, IRG, IRF, IRM, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        functions::inventions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, RG, RF, RM, AU, IAU, IRG, IRF, IRM,
        >,
    >,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    IAU: functions::inventions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
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
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());
        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`.
        // See `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(chunk) = stream.next().await {
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn create_function_invention_recursive_ws<
    OR, CAG, CX, MK, RG, RF, RM, AU, IAU, IRG, IRF, IRM, RAU, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        functions::inventions::recursive::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, RG, RF, RM, AU, IAU, IRG, IRF, IRM, RAU,
        >,
    >,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    IAU: functions::inventions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    IRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RAU: functions::inventions::recursive::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
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
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let body: objectiveai_sdk::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());
        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`.
        // See `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client.create_streaming_handle_usage(ctx, Arc::new(body)).await {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(chunk) = stream.next().await {
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn create_error_ws<NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>(
    client: Arc<crate::error::Client>,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
        // Error stream doesn't carry agent-completion ids; tracker stays
        // empty, every incoming notify will validation-fail with 404.
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());
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
                let mut guard = send_sink.lock().await;
                if guard
                    .send(axum::extract::ws::Message::Text(frame.into()))
                    .await
                    .is_err()
                {
                    return;
                }
                drop(guard);
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}

pub(crate) async fn execute_laboratory_ws<
    OR, CAG, CX, MK, RG, RF, RM, AU, LAU, ORC, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU,
>(
    client: Arc<
        crate::laboratories::executions::Client<
            ctx::DefaultContextExt,
            OR, CAG, CX, MK, RG, RF, RM, AU, LAU, ORC,
        >,
    >,
    agent_completions_client: Arc<
        agent::completions::Client<ctx::DefaultContextExt, NOR, NCAG, NCX, NMK, NRG, NRF, NRM, NAU>,
    >,
    reverse_attach: streaming_ws::ReverseAttachConfig,
    headers: axum::http::HeaderMap,
    persistent_cache: Arc<impl ctx::persistent_cache::PersistentCacheClient + 'static>,
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
    RG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    RM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    AU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    LAU: crate::laboratories::executions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
    ORC: crate::laboratories::orchestrator::Orchestrator<ctx::DefaultContextExt> + Send + Sync + 'static,
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
    NRG: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRF: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NRM: retrieval::retrieve::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    NAU: agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt> + Send + Sync + 'static,
{
    ws.on_upgrade(move |mut socket| async move {
        let request: objectiveai_sdk::laboratories::executions::request::LaboratoryExecutionCreateParams =
            match streaming_ws::recv_body_frame(&mut socket).await {
                Ok(b) => b,
                Err(err) => {
                    streaming_ws::send_error_and_close(&mut socket, &err, close_code::UNSUPPORTED)
                        .await;
                    return;
                }
            };
        let tracker = streaming_ws::SessionTracker::new();
        let pending = streaming_ws::new_pending_requests();
        let (tx, rx) = socket.split();
        let sink: streaming_ws::SharedSink = Arc::new(tokio::sync::Mutex::new(tx));
        let _attach_guard = streaming_ws::ReverseAttachGuard::new(
            reverse_attach.registry.clone(),
            sink.clone(),
            pending.clone(),
        );
        let ctx = crate::context(&headers, persistent_cache, suppress_output)
            .with_mcp_port(reverse_attach.mcp_port)
            .with_reverse_attach(_attach_guard.handle());
        // Stream setup lives INSIDE the `send` branch so `recv_loop`
        // is polled concurrently with `create_streaming_handle_usage`.
        // See `create_vector_completion_ws` for rationale.
        let send_sink = sink.clone();
        let send_tracker = tracker.clone();
        let send = async move {
            let stream = match client
                .create_streaming_handle_usage(ctx, Arc::new(request))
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    streaming_ws::fatal_setup_error_split(
                        &send_sink,
                        &ResponseError::from(&e),
                    )
                    .await;
                    return;
                }
            };
            let mut stream = Box::pin(stream);
            while let Some(chunk) = stream.next().await {
                send_tracker.observe(&chunk);
                if streaming_ws::send_chunk_split(&send_sink, &chunk).await.is_err() {
                    return;
                }
            }
            streaming_ws::send_close_split(&send_sink, close_code::NORMAL).await;
        };

        let recv = streaming_ws::recv_loop(
            rx,
            tracker,
            sink,
            pending,
            reverse_attach.mcp_listeners.clone(),
            _attach_guard.handle(),
            notify_fn_for(agent_completions_client),
        );

        tokio::select! {
            _ = send => {},
            _ = recv => {},
        }
    })
}
