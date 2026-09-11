//! Process-wide shared test clients.
//!
//! Every test in this crate reuses these singletons. Each layered
//! client embeds its dependencies as `Arc<_>`, so sharing the root
//! agent client transitively shares the proxy spawner and every
//! stub. That collapses the
//! per-test loopback listener count from O(770+) to O(1) per
//! `cargo test` process — eliminating the WinSock SYN-backlog and
//! ephemeral-port exhaustion that surfaced as flaky `ECONNREFUSED` /
//! `ECONNABORTED` / `ECONNRESET` under parallel load.

use std::sync::{Arc, LazyLock};
use std::time::Duration;

use crate::ctx;

// ---------------------------------------------------------------------------
// Stub retrieve client — never called; tests always provide inline data.
// ---------------------------------------------------------------------------

pub(crate) struct StubRetrieveClient;

#[async_trait::async_trait]
impl crate::retrieval::retrieve::Client<ctx::DefaultContextExt> for StubRetrieveClient {
    async fn get_agent(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt>,
        _path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::agent::RemoteAgentBaseWithFallbacks>, objectiveai_sdk::error::ResponseError> {
        Err(objectiveai_sdk::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_swarm(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt>,
        _path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::swarm::RemoteSwarmBase>, objectiveai_sdk::error::ResponseError> {
        Err(objectiveai_sdk::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_function(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt>,
        _path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::FullRemoteFunction>, objectiveai_sdk::error::ResponseError> {
        Err(objectiveai_sdk::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_profile(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt>,
        _path: &objectiveai_sdk::RemotePath,
    ) -> Result<Option<objectiveai_sdk::functions::RemoteProfile>, objectiveai_sdk::error::ResponseError> {
        Err(objectiveai_sdk::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn resolve_latest(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt>,
        _kind: crate::retrieval::Kind,
        _path: &objectiveai_sdk::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai_sdk::RemotePath>, objectiveai_sdk::error::ResponseError> {
        Err(objectiveai_sdk::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }
}

// ---------------------------------------------------------------------------
// Stub usage handlers — no-ops at every layer.
// ---------------------------------------------------------------------------

pub(crate) struct StubAgentUsageHandler;

impl crate::agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubAgentUsageHandler
{
    fn handle_usage(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt>,
        _request: Arc<objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams>,
        _response: objectiveai_sdk::agent::completions::response::unary::AgentCompletion,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        async {}
    }
}

pub(crate) struct StubVectorUsageHandler;

#[async_trait::async_trait]
impl crate::vector::completions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubVectorUsageHandler
{
    async fn handle_usage(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt>,
        _request: Arc<objectiveai_sdk::vector::completions::request::VectorCompletionCreateParams>,
        _response: objectiveai_sdk::vector::completions::response::unary::VectorCompletion,
    ) {
    }
}

pub(crate) struct StubFunctionUsageHandler;

#[async_trait::async_trait]
impl crate::functions::executions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubFunctionUsageHandler
{
    async fn handle_usage(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt>,
        _request: Arc<objectiveai_sdk::functions::executions::request::FunctionExecutionCreateParams>,
        _response: objectiveai_sdk::functions::executions::response::unary::FunctionExecution,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Concrete-type aliases.
// ---------------------------------------------------------------------------

use crate::agent::completions::UnimplementedUpstreamClient;

pub(crate) type AgentRetrieveRouter = crate::retrieval::retrieve::Router<
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    ctx::DefaultContextExt,
>;

pub(crate) type FunctionRetrieveRouter = crate::retrieval::retrieve::Router<
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    ctx::DefaultContextExt,
>;

pub(crate) type AgentClient = crate::agent::completions::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    crate::agent::completions::script::Client,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubAgentUsageHandler,
>;

pub(crate) type VectorClient = crate::vector::completions::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    crate::agent::completions::script::Client,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubAgentUsageHandler,
    StubVectorUsageHandler,
>;

pub(crate) type FunctionExecutionsClient = crate::functions::executions::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    crate::agent::completions::script::Client,
    StubAgentUsageHandler,
    StubVectorUsageHandler,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubFunctionUsageHandler,
>;

// ---------------------------------------------------------------------------
// Process-wide background runtime.
//
// Every long-lived listener task (the proxy's `axum::serve`) lives
// on this runtime so it survives across
// `#[tokio::test]` runtimes that drop at end-of-test. Without this, the
// first test to call `proxy_spawner().get().await` would anchor the
// listener task to its own runtime; that runtime drops; the listener task
// is silently aborted; every subsequent test reads the cached
// `Arc<ProxyHandle>` from the OnceCell, gets the same URL, and TCP-connects
// → ECONNREFUSED.
// ---------------------------------------------------------------------------

static BACKGROUND_RUNTIME: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .thread_name("test-clients-bg")
        .build()
        .expect("build test_clients background runtime")
});

// ---------------------------------------------------------------------------
// Process-wide cap on how many `#[tokio::test]`s in this crate may be in
// flight at once. Read from `TOKIO_TEST_PARALLELISM`; default 10. The
// bound exists because all tests share one in-process MCP proxy
// (the singletons in this file), so they also share the
// same `(127.0.0.1, proxy_port)` outbound 4-tuple — uncapped parallelism
// saturates Windows' ephemeral source-port range and surfaces as
// `WSAEADDRINUSE` (10048). Bound parallelism = bound port churn.
// ---------------------------------------------------------------------------

static TEST_PARALLELISM_SEMAPHORE: LazyLock<Arc<tokio::sync::Semaphore>> = LazyLock::new(|| {
    let limit: usize = match std::env::var("TOKIO_TEST_PARALLELISM") {
        Ok(s) => s.parse().unwrap_or_else(|e| {
            panic!("TOKIO_TEST_PARALLELISM must parse as a positive integer: {e}");
        }),
        Err(_) => 10,
    };
    Arc::new(tokio::sync::Semaphore::new(limit))
});

/// Acquire one permit from the test parallelism semaphore. Hold the
/// returned guard for the duration of the test; drop it (let it fall
/// off the end of the function or assign to `_permit`) to release.
pub(crate) async fn acquire_test_permit() -> tokio::sync::OwnedSemaphorePermit {
    TEST_PARALLELISM_SEMAPHORE
        .clone()
        .acquire_owned()
        .await
        .expect("test parallelism semaphore unexpectedly closed")
}

// ---------------------------------------------------------------------------
// Singletons. Every accessor returns `Arc::clone` of the same instance —
// no per-call construction, ever.
// ---------------------------------------------------------------------------

static MOCK_UPSTREAM: LazyLock<Arc<crate::agent::completions::mock::Client>> = LazyLock::new(|| {
    Arc::new(crate::agent::completions::mock::Client {
        delay: Duration::ZERO,
        max_tool_calls: 1000,
    })
});

static AGENT_RETRIEVE_ROUTER: LazyLock<Arc<AgentRetrieveRouter>> = LazyLock::new(|| {
    Arc::new(crate::retrieval::retrieve::Router::new(
        STUB_RETRIEVE_CLIENT.clone(),
        STUB_RETRIEVE_CLIENT.clone(),
        MOCK_RETRIEVE_CLIENT.clone(),
    ))
});

// MCP backoff + timeout config used by both the singleton MCP client
// (api → proxy) and the in-process proxy spawner (proxy → upstream).
//
// These do NOT mirror production. Production
// (`objectiveai-api/src/run.rs::ConfigBuilder::build`) has no call
// timeout at all — `mcp_call_timeout` defaults to `None`, and the env
// that once set it was removed in 6f95884c9 ("wait forever on the CLI
// client") — and its connect timeout is 30 minutes. Tests pin finite
// values instead, because an unbounded wait turns a hung mock into a
// hung suite, and fixed backoff numbers keep the retry policy
// deterministic.
//
// `MCP_CALL_TIMEOUT_MS` is 60s because single test slots under the
// full concurrent integration suite have been observed to lose ~40s+
// of wall time to scheduler pressure mid-`list_tools`; a shorter
// budget surfaces as a flaky "operation timed out" on whichever seed
// gets unlucky, while a genuinely >60s `list_tools` against the local
// mock would still surface as the regression it is.
const MCP_CONNECT_TIMEOUT_MS: u64 = 30_000;
pub(crate) const MCP_CALL_TIMEOUT_MS: u64 = 60_000;
const MCP_BACKOFF_CURRENT_INTERVAL_MS: u64 = 100;
const MCP_BACKOFF_INITIAL_INTERVAL_MS: u64 = 100;
const MCP_BACKOFF_RANDOMIZATION_FACTOR: f64 = 0.5;
const MCP_BACKOFF_MULTIPLIER: f64 = 1.5;
const MCP_BACKOFF_MAX_INTERVAL_MS: u64 = 1_000;
const MCP_BACKOFF_MAX_ELAPSED_TIME_MS: u64 = 40_000;

static MCP_CLIENT: LazyLock<Arc<objectiveai_sdk::mcp::Client>> = LazyLock::new(|| {
    // Construct reqwest::Client inside the BACKGROUND_RUNTIME so the
    // hyper connection-pool dispatch tasks live forever. Constructing
    // it on a per-`#[tokio::test]` runtime binds the client's HTTP
    // connection pool to that runtime — once the test ends and its
    // runtime drops, every cached connection's dispatch task dies and
    // subsequent tests using this `LazyLock` see
    // `runtime dropped the dispatch task` errors on POSTs to the
    // proxy, which manifest as parallel-only test flakes.
    let _guard = BACKGROUND_RUNTIME.handle().enter();
    let reqwest = reqwest::Client::builder()
        .build()
        .expect("build reqwest::Client");
    drop(_guard);
    Arc::new(objectiveai_sdk::mcp::Client::new(
        reqwest,
        String::new(),
        String::new(),
        String::new(),
        Some(Duration::from_millis(MCP_CONNECT_TIMEOUT_MS)),
        Duration::from_millis(MCP_BACKOFF_CURRENT_INTERVAL_MS),
        Duration::from_millis(MCP_BACKOFF_INITIAL_INTERVAL_MS),
        MCP_BACKOFF_RANDOMIZATION_FACTOR,
        MCP_BACKOFF_MULTIPLIER,
        Duration::from_millis(MCP_BACKOFF_MAX_INTERVAL_MS),
        Duration::from_millis(MCP_BACKOFF_MAX_ELAPSED_TIME_MS),
        Some(Duration::from_millis(MCP_CALL_TIMEOUT_MS)),
    ))
});

static PROXY_SPAWNER: LazyLock<Arc<crate::agent::completions::ProxyFactory>> = LazyLock::new(|| {
    Arc::new(crate::agent::completions::ProxyFactory::new_with_handle(
        BACKGROUND_RUNTIME.handle().clone(),
        || objectiveai_mcp_proxy::ConfigBuilder {
            mcp_connect_timeout: Some(MCP_CONNECT_TIMEOUT_MS),
            mcp_call_timeout: Some(MCP_CALL_TIMEOUT_MS),
            mcp_backoff_max_elapsed_time: Some(MCP_BACKOFF_MAX_ELAPSED_TIME_MS),
            ..Default::default()
        },
    ))
});

// --- single shared instances of every Stub / Unimplemented / Mock client ---

static STUB_RETRIEVE_CLIENT: LazyLock<Arc<StubRetrieveClient>> =
    LazyLock::new(|| Arc::new(StubRetrieveClient));
static MOCK_RETRIEVE_CLIENT: LazyLock<Arc<crate::retrieval::retrieve::mock::MockClient>> =
    LazyLock::new(|| Arc::new(crate::retrieval::retrieve::mock::MockClient));
static STUB_AGENT_USAGE_HANDLER: LazyLock<Arc<StubAgentUsageHandler>> =
    LazyLock::new(|| Arc::new(StubAgentUsageHandler));
static STUB_VECTOR_USAGE_HANDLER: LazyLock<Arc<StubVectorUsageHandler>> =
    LazyLock::new(|| Arc::new(StubVectorUsageHandler));
static STUB_FUNCTION_USAGE_HANDLER: LazyLock<Arc<StubFunctionUsageHandler>> =
    LazyLock::new(|| Arc::new(StubFunctionUsageHandler));
static UNIMPLEMENTED_OPENROUTER: LazyLock<Arc<UnimplementedUpstreamClient>> =
    LazyLock::new(|| Arc::new(UnimplementedUpstreamClient));
static UNIMPLEMENTED_CLAUDE_AGENT_SDK: LazyLock<Arc<UnimplementedUpstreamClient>> =
    LazyLock::new(|| Arc::new(UnimplementedUpstreamClient));
static UNIMPLEMENTED_CODEX_SDK: LazyLock<Arc<UnimplementedUpstreamClient>> =
    LazyLock::new(|| Arc::new(UnimplementedUpstreamClient));

static SCRIPT_UPSTREAM: LazyLock<Arc<crate::agent::completions::script::Client>> =
    LazyLock::new(|| Arc::new(crate::agent::completions::script::Client));

// --- the API client singletons, each constructed once, ever ---

static AGENT: LazyLock<Arc<AgentClient>> = LazyLock::new(|| {
    Arc::new(crate::agent::completions::Client::new(
        MCP_CLIENT.clone(),
        PROXY_SPAWNER.clone(),
        None,
        AGENT_RETRIEVE_ROUTER.clone(),
        STUB_AGENT_USAGE_HANDLER.clone(),
        UNIMPLEMENTED_OPENROUTER.clone(),
        UNIMPLEMENTED_CLAUDE_AGENT_SDK.clone(),
        UNIMPLEMENTED_CODEX_SDK.clone(),
        MOCK_UPSTREAM.clone(),
        SCRIPT_UPSTREAM.clone(),
        Duration::ZERO,
        Duration::ZERO,
        0.0,
        1.0,
        Duration::ZERO,
        Duration::ZERO,
        Duration::from_secs(1800),
    ))
});

static VECTOR: LazyLock<Arc<VectorClient>> = LazyLock::new(|| {
    Arc::new(crate::vector::completions::Client::new(
        AGENT.clone(),
        AGENT_RETRIEVE_ROUTER.clone(),
        STUB_VECTOR_USAGE_HANDLER.clone(),
    ))
});

static FUNCTION_EXECUTIONS: LazyLock<Arc<FunctionExecutionsClient>> = LazyLock::new(|| {
    Arc::new(crate::functions::executions::Client::new(
        AGENT.clone(),
        VECTOR.clone(),
        AGENT_RETRIEVE_ROUTER.clone(),
        STUB_FUNCTION_USAGE_HANDLER.clone(),
    ))
});

// ---------------------------------------------------------------------------
// Public accessors. Every accessor returns `Arc::clone` of the static
// singleton — never reconstructs.
// ---------------------------------------------------------------------------

pub(crate) fn mock_upstream() -> Arc<crate::agent::completions::mock::Client> {
    MOCK_UPSTREAM.clone()
}

pub(crate) fn proxy_spawner() -> Arc<crate::agent::completions::ProxyFactory> {
    PROXY_SPAWNER.clone()
}

/// Drive `fut` on the long-lived `BACKGROUND_RUNTIME` instead of a per-
/// `#[test]` runtime. Required for any test that exercises the
/// proxy pipeline because reqwest's hyper connection pool
/// spawns its dispatch tasks on whatever runtime is current when a
/// connection is established and reused. A short-lived per-test runtime
/// drops mid-flight, killing pooled dispatch tasks the instant another
/// test (or even a later step of the same test running on a different
/// thread) tries to send on those connections — surfacing as a
/// "request error … client error (SendRequest): dispatch task is gone:
/// runtime dropped the dispatch task" error and producing parallel-only
/// flakes that disappear under solo execution.
pub(crate) fn run_test<F: std::future::Future>(fut: F) -> F::Output {
    BACKGROUND_RUNTIME.block_on(fut)
}

pub(crate) fn mcp_client() -> Arc<objectiveai_sdk::mcp::Client> {
    MCP_CLIENT.clone()
}

pub(crate) fn agent() -> Arc<AgentClient> {
    AGENT.clone()
}

pub(crate) fn vector() -> Arc<VectorClient> {
    VECTOR.clone()
}

pub(crate) fn function_executions() -> Arc<FunctionExecutionsClient> {
    FUNCTION_EXECUTIONS.clone()
}

