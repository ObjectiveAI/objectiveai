//! Tests for recursive function invention client.

use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;

use objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams;
use objectiveai::functions::inventions::recursive::response::unary::FunctionInventionRecursive;
use objectiveai::functions::inventions::state::{Params, ParamsState};
use objectiveai::functions::inventions::state::{
    AlphaScalarLeafState, AlphaScalarBranchState,
    AlphaVectorLeafState, AlphaVectorBranchState,
    AlphaScalarState, AlphaVectorState,
};

use crate::agent::completions::UnimplementedUpstreamClient;
use crate::ctx;

// ---------------------------------------------------------------------------
// Stubs
// ---------------------------------------------------------------------------

struct StubRetrieveClient;

#[async_trait::async_trait]
impl crate::retrieval::retrieve::Client<ctx::DefaultContextExt> for StubRetrieveClient {
    async fn get_agent<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::agent::RemoteAgentBaseWithFallbacks>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
    async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::swarm::RemoteSwarmBase>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
    async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::FullRemoteFunction>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
    async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::RemoteProfile>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
    async fn get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::inventions::prompts::RemotePrompt>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
    async fn get_function_invention_state_file<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
        _filename: &'static str,
    ) -> Result<Option<String>, objectiveai::error::ResponseError> {
        unimplemented!()
    }

    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _kind: crate::retrieval::Kind,
        _path: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::RemotePath>, objectiveai::error::ResponseError> {
        unimplemented!()
    }
}

struct StubAgentUsageHandler;

impl crate::agent::completions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubAgentUsageHandler
{
    fn handle_usage(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        _request: Arc<objectiveai::agent::completions::request::AgentCompletionCreateParams>,
        _response: objectiveai::agent::completions::response::unary::AgentCompletion,
    ) -> impl std::future::Future<Output = ()> + Send + 'static {
        async {}
    }
}

struct StubInventionUsageHandler;

#[async_trait::async_trait]
impl crate::functions::inventions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubInventionUsageHandler
{
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _request: Arc<objectiveai::functions::inventions::request::FunctionInventionCreateParams>,
        _response: objectiveai::functions::inventions::response::unary::FunctionInvention,
    ) {
    }
}

struct StubRecursiveUsageHandler;

#[async_trait::async_trait]
impl super::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubRecursiveUsageHandler
{
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _request: Arc<FunctionInventionRecursiveCreateParams>,
        _response: FunctionInventionRecursive,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

type TestInventionClient = crate::functions::inventions::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    StubRetrieveClient,
    StubRetrieveClient,
    StubRetrieveClient,
    StubAgentUsageHandler,
    StubInventionUsageHandler,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
>;

type TestClient = super::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubAgentUsageHandler,
    StubInventionUsageHandler,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubRecursiveUsageHandler,
>;

fn make_client() -> Arc<TestClient> {
    let retrieve_router = Arc::new(crate::retrieval::retrieve::Router::new(
        Arc::new(StubRetrieveClient),
        Arc::new(StubRetrieveClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
    ));
    let agent_client = Arc::new(crate::agent::completions::Client::new(
        Arc::new(objectiveai::mcp::Client::new(
            reqwest::Client::new(),
            String::new(),
            String::new(),
            String::new(),
            Duration::from_millis(1),
            Duration::ZERO,
            Duration::ZERO,
            0.0,
            1.0,
            Duration::ZERO,
            Duration::ZERO,
            Duration::from_millis(1),
        )),
        None, // mcp_authorization
        retrieve_router.clone(),
        Arc::new(StubAgentUsageHandler),
        Arc::new(UnimplementedUpstreamClient),
        Arc::new(UnimplementedUpstreamClient),
        Arc::new(crate::agent::completions::mock::Client {
            delay: Duration::ZERO,
            max_tool_calls: 1000,
        }),
        Arc::new(crate::viewer::Client::new(
            reqwest::Client::new(), None, None,
            std::time::Duration::ZERO, std::time::Duration::ZERO, 0.0, 1.0,
            std::time::Duration::ZERO, std::time::Duration::from_millis(1),
        )),
        Duration::ZERO,
        Duration::ZERO,
        0.0,
        1.0,
        Duration::ZERO,
        Duration::ZERO,
        Duration::from_millis(1),
        Duration::from_millis(1),
    ));
    let github_client = Arc::new(crate::github::Client::new(
        reqwest::Client::new(),
        None,
        false,
        String::new(),
        String::new(),
        String::new(),
        Duration::ZERO,
        Duration::ZERO,
        0.0,
        1.0,
        Duration::ZERO,
        Duration::ZERO,
    ));
    let filesystem_client = Arc::new(crate::filesystem::Client::new(
        std::path::PathBuf::from("/tmp/objectiveai-test-recursive"),
        "ObjectiveAI".to_string(),
        "noreply@objectiveai.dev".to_string(),
    ));
    let function_retrieve_router = Arc::new(crate::retrieval::retrieve::Router::new(
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
    ));
    let invention_client = Arc::new(crate::functions::inventions::Client::new(
        agent_client,
        github_client,
        filesystem_client,
        function_retrieve_router,
        Arc::new(StubInventionUsageHandler),
        true,
        false,
    ));
    let viewer_client = Arc::new(crate::viewer::Client::new(
        reqwest::Client::new(),
        None,
        None,
        Duration::ZERO,
        Duration::ZERO,
        0.0,
        1.0,
        Duration::ZERO,
        Duration::ZERO,
    ));
    Arc::new(super::Client::new(
        invention_client,
        viewer_client,
        Arc::new(StubRecursiveUsageHandler),
    ))
}

fn make_request(state: ParamsState, seed: i64) -> Arc<FunctionInventionRecursiveCreateParams> {
    Arc::new(FunctionInventionRecursiveCreateParams {
        remote: objectiveai::Remote::Mock,
        overwrite: None,
        state: objectiveai::functions::inventions::ParamsStateOrRemoteCommitOptional::Inline(state),
        provider: None,
        agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase {
                    mode: Some(objectiveai::agent::mock::Mode::Invention),
                    ..Default::default()
                }),
                fallbacks: None,
            },
        ),
        prompt: objectiveai::functions::inventions::prompts::InlinePromptOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "default".to_string() },
        ),
        seed: Some(seed),
        stream: Some(true),
        max_step_retries: Some(1),
        continuation: None,
    })
}

fn params(name: &str, depth: u64, min_b: u64, max_b: u64, min_l: u64, max_l: u64) -> Params {
    Params {
        depth,
        min_branch_width: min_b,
        max_branch_width: max_b,
        min_leaf_width: min_l,
        max_leaf_width: max_l,
        name: name.to_string(),
        spec: "Test function spec for mock recursive invention.".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Streaming + aggregation helpers
// ---------------------------------------------------------------------------

fn check_created(expected: &std::cell::Cell<Option<u64>>, i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(exp) => assert_eq!(created, exp, "chunk {i} has created {created}, expected {exp}"),
    }
}

async fn run_recursive_invention(
    client: &Arc<TestClient>,
    request: Arc<FunctionInventionRecursiveCreateParams>,
) -> FunctionInventionRecursive {
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let stream = client
        .clone()
        .create_streaming(ctx, request)
        .await
        .expect("create_streaming should succeed");
    let expected_created = std::cell::Cell::new(None);
    let agg = crate::stream_harness::consume_stream(
        Box::pin(stream),
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 1, "chunk {i} (non-final) has {} invention chunks, expected exactly 1", chunk.inventions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 1, "chunk {i} (non-final) has {} invention chunks, expected exactly 1", chunk.inventions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert_eq!(chunk.inventions.len(), 0, "final chunk {i} has {} invention chunks, expected 0", chunk.inventions.len());
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionInventionRecursive::from(agg)
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn normalize(mut fi: FunctionInventionRecursive) -> FunctionInventionRecursive {
    fi.normalize_for_tests();
    fi
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    crate::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_INVENTIONS_RECURSIVE_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Test macro — 3 seeds per test (recursive tests are heavier)
// ---------------------------------------------------------------------------

macro_rules! recursive_test_3x {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr
    ) => {
        mod $test_name {
            use super::*;

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        essay: None,
                        input_schema: None,
                        essay_tasks: None,
                        tasks: None,
                        tasks_length: None,
                        description: None,
                        readme: None,
                        checker_seed: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let client = make_client();
                    let (state, seed) = make_state(offset);
                    let request = make_request(state, seed);
                    let result = normalize(run_recursive_invention(&client, request).await);
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    assert_snapshot(&json, path, expected);
                });
            }

            #[test]
            fn seed_0() {
                run_snapshot(
                    0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_0.json")),
                );
            }

            #[test]
            fn seed_1() {
                run_snapshot(
                    1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_1.json")),
                );
            }

            #[test]
            fn seed_2() {
                run_snapshot(
                    2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_2.json")),
                );
            }
        }
    };
}

/// Same as `recursive_test_3x!` but uses AlphaScalar/AlphaVector (unrouted state).
macro_rules! recursive_test_3x_unrouted {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr
    ) => {
        mod $test_name {
            use super::*;

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        input_schema: None,
                    }),
                    ($base_seed as i64) + seed_offset,
                )
            }

            fn run_snapshot(offset: i64, path: &str, expected: &str) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let client = make_client();
                    let (state, seed) = make_state(offset);
                    let request = make_request(state, seed);
                    let result = normalize(run_recursive_invention(&client, request).await);
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    assert_snapshot(&json, path, expected);
                });
            }

            #[test]
            fn seed_0() {
                run_snapshot(
                    0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_0.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_0.json")),
                );
            }

            #[test]
            fn seed_1() {
                run_snapshot(
                    1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_1.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_1.json")),
                );
            }

            #[test]
            fn seed_2() {
                run_snapshot(
                    2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/", $base, "_2.json"),
                    include_str!(concat!("../../../../assets/functions/inventions/recursive_client_tests/", $base, "_2.json")),
                );
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Leaf tests (depth=0) — just 2, since recursive is wasteful at depth 0
// ---------------------------------------------------------------------------

recursive_test_3x!(test_scalar_leaf_d0,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "rsl-baseline", 0, 1, 1, 2, 4, 100,
    "scalar_leaf_d0");

recursive_test_3x!(test_vector_leaf_d0,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "rvl-baseline", 0, 1, 1, 2, 4, 200,
    "vector_leaf_d0");

// ---------------------------------------------------------------------------
// Depth 1 — scalar (diverse widths and configs)
// ---------------------------------------------------------------------------

// Scalar branch, depth 1, minimum: 1 branch task, 1 leaf task
recursive_test_3x!(test_scalar_d1_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-min", 1, 1, 1, 1, 1, 1000,
    "scalar_d1_min");

// Scalar branch, depth 1, default widths 3-5
recursive_test_3x!(test_scalar_d1_default,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-default", 1, 3, 5, 3, 5, 1100,
    "scalar_d1_default");

// Scalar branch, depth 1, narrow branch + wide leaf
recursive_test_3x!(test_scalar_d1_narrow_branch_wide_leaf,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-nbwl", 1, 1, 2, 6, 8, 1200,
    "scalar_d1_narrow_branch_wide_leaf");

// Scalar branch, depth 1, wide branch + narrow leaf
recursive_test_3x!(test_scalar_d1_wide_branch_narrow_leaf,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-wbnl", 1, 6, 8, 1, 2, 1300,
    "scalar_d1_wide_branch_narrow_leaf");

// Scalar branch, depth 1, exact 4 tasks each
recursive_test_3x!(test_scalar_d1_exact_4,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-exact4", 1, 4, 4, 4, 4, 1400,
    "scalar_d1_exact_4");

// Scalar, depth 1, unrouted (AlphaScalar routes to branch)
recursive_test_3x_unrouted!(test_scalar_d1_unrouted,
    AlphaScalar, AlphaScalarState,
    "rs-d1-unrouted", 1, 2, 3, 2, 3, 1500,
    "scalar_d1_unrouted");

// ---------------------------------------------------------------------------
// Depth 1 — vector (diverse widths and configs)
// ---------------------------------------------------------------------------

// Vector branch, depth 1, minimum widths
recursive_test_3x!(test_vector_d1_min,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-min", 1, 1, 1, 1, 1, 2000,
    "vector_d1_min");

// Vector branch, depth 1, default widths
recursive_test_3x!(test_vector_d1_default,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-default", 1, 3, 5, 3, 5, 2100,
    "vector_d1_default");

// Vector branch, depth 1, wide branch + narrow leaf
recursive_test_3x!(test_vector_d1_wide_branch,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-wb", 1, 5, 8, 1, 2, 2200,
    "vector_d1_wide_branch");

// Vector branch, depth 1, narrow branch + wide leaf
recursive_test_3x!(test_vector_d1_narrow_branch,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-nb", 1, 1, 2, 5, 8, 2300,
    "vector_d1_narrow_branch");

// Vector branch, depth 1, exact 3
recursive_test_3x!(test_vector_d1_exact_3,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-exact3", 1, 3, 3, 3, 3, 2400,
    "vector_d1_exact_3");

// Vector, depth 1, unrouted (AlphaVector routes to branch)
recursive_test_3x_unrouted!(test_vector_d1_unrouted,
    AlphaVector, AlphaVectorState,
    "rv-d1-unrouted", 1, 2, 4, 2, 4, 2500,
    "vector_d1_unrouted");

// Vector branch, depth 1, asymmetric range 1-10
recursive_test_3x!(test_vector_d1_wide_range,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-range", 1, 1, 10, 1, 10, 2600,
    "vector_d1_wide_range");

// ---------------------------------------------------------------------------
// Depth 2 — scalar and vector
// ---------------------------------------------------------------------------

// Scalar branch, depth 2, narrow (2-3 tasks per level)
recursive_test_3x!(test_scalar_d2_narrow,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d2-narrow", 2, 2, 3, 2, 3, 3000,
    "scalar_d2_narrow");

// Scalar branch, depth 2, minimum
recursive_test_3x!(test_scalar_d2_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d2-min", 2, 1, 1, 1, 1, 3100,
    "scalar_d2_min");

// Vector branch, depth 2, default widths
recursive_test_3x!(test_vector_d2_default,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d2-default", 2, 3, 5, 3, 5, 3200,
    "vector_d2_default");

// Vector branch, depth 2, narrow
recursive_test_3x!(test_vector_d2_narrow,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d2-narrow", 2, 2, 3, 2, 3, 3300,
    "vector_d2_narrow");

// Scalar, depth 2, unrouted
recursive_test_3x_unrouted!(test_scalar_d2_unrouted,
    AlphaScalar, AlphaScalarState,
    "rs-d2-unrouted", 2, 1, 2, 1, 2, 3400,
    "scalar_d2_unrouted");

// ---------------------------------------------------------------------------
// Depth 3 — just one (expensive)
// ---------------------------------------------------------------------------

// Scalar branch, depth 3, minimum widths to keep it tractable
recursive_test_3x!(test_scalar_d3_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d3-min", 3, 1, 1, 1, 1, 4000,
    "scalar_d3_min");

// ---------------------------------------------------------------------------
// Initial state validation tests
// ---------------------------------------------------------------------------

use objectiveai::functions::expression::{InputSchema, ObjectInputSchema, StringInputSchema};
use objectiveai::functions::alpha_scalar;
use objectiveai::functions::alpha_vector;
use indexmap::IndexMap;

/// Helper: run a recursive invention that is expected to produce an error.
/// The recursive client returns validation errors as Err from create_streaming.
async fn run_recursive_invention_err(
    client: &Arc<TestClient>,
    request: Arc<FunctionInventionRecursiveCreateParams>,
) -> String {
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    match client.clone().create_streaming(ctx, request).await {
        Err(err) => err.to_string(),
        Ok(_) => panic!("create_streaming should return Err for invalid state"),
    }
}

/// A valid scalar input schema: object with a required string enum of 2 values.
fn valid_scalar_schema() -> alpha_scalar::expression::ScalarFunctionInputSchema {
    let mut properties = IndexMap::new();
    properties.insert(
        "sentiment".to_string(),
        InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["positive".to_string(), "negative".to_string()]),
        }),
    );
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties,
        required: Some(vec!["sentiment".to_string()]),
    }
}

/// An invalid scalar input schema: single enum value → only 1 permutation.
fn invalid_scalar_schema() -> alpha_scalar::expression::ScalarFunctionInputSchema {
    let mut properties = IndexMap::new();
    properties.insert(
        "mood".to_string(),
        InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["sad".to_string()]),
        }),
    );
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties,
        required: Some(vec!["mood".to_string()]),
    }
}

/// A valid vector input schema: items is a string enum of 2 values.
fn valid_vector_schema() -> alpha_vector::expression::VectorFunctionInputSchema {
    alpha_vector::expression::VectorFunctionInputSchema {
        context: None,
        items: InputSchema::String(StringInputSchema {
            r#type: Default::default(),
            description: None,
            r#enum: Some(vec!["apple".to_string(), "banana".to_string()]),
        }),
    }
}

/// A valid scalar leaf task: vector completion with messages derived from input.
fn valid_scalar_leaf_task() -> alpha_scalar::LeafTaskExpression {
    alpha_scalar::LeafTaskExpression::VectorCompletion(
        alpha_scalar::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": str(input)}]}]".to_string(),
            ),
            responses: vec![
                objectiveai::agent::completions::message::RichContent::Parts(vec![
                    objectiveai::agent::completions::message::RichContentPart::Text { text: "yes".to_string() },
                ]),
                objectiveai::agent::completions::message::RichContent::Parts(vec![
                    objectiveai::agent::completions::message::RichContentPart::Text { text: "no".to_string() },
                ]),
            ],
        },
    )
}

/// An invalid scalar leaf task: messages is a fixed string (not derived from input).
fn invalid_scalar_leaf_task() -> alpha_scalar::LeafTaskExpression {
    alpha_scalar::LeafTaskExpression::VectorCompletion(
        alpha_scalar::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"hardcoded\"}]}]".to_string(),
            ),
            responses: vec![
                objectiveai::agent::completions::message::RichContent::Parts(vec![
                    objectiveai::agent::completions::message::RichContentPart::Text { text: "yes".to_string() },
                ]),
                objectiveai::agent::completions::message::RichContent::Parts(vec![
                    objectiveai::agent::completions::message::RichContentPart::Text { text: "no".to_string() },
                ]),
            ],
        },
    )
}

/// A valid vector leaf task: messages and responses derived from input.
fn valid_vector_leaf_task() -> alpha_vector::LeafTaskExpression {
    alpha_vector::LeafTaskExpression::VectorCompletion(
        alpha_vector::VectorCompletionTaskExpression {
            skip: None,
            messages: objectiveai::functions::expression::Expression::Starlark(
                "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"rank these\"}]}]".to_string(),
            ),
            responses: objectiveai::functions::expression::Expression::Starlark(
                "[[{\"type\": \"text\", \"text\": str(item)}] for item in input['items']]".to_string(),
            ),
        },
    )
}

// --- Error tests: invalid initial states ---

#[test]
fn test_invalid_scalar_input_schema() {
    // Scalar leaf with an invalid input schema (only 1 permutation) and no tasks.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-bad-schema", 0, 1, 1, 2, 4),
            essay: Some("An essay about feelings.".to_string()),
            input_schema: Some(invalid_scalar_schema()),
            essay_tasks: None,
            tasks: None,
            tasks_length: None,
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5000);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
    });
}

// test_invalid_vector_input_schema removed: VectorFunctionInputSchema.transpile()
// now wraps items in ArrayInputSchema(min_items=2), so a single-enum items
// schema produces enough array permutations to pass QI01.

#[test]
fn test_valid_schema_invalid_tasks_scalar_leaf() {
    // Scalar leaf with a valid input schema but tasks that don't use the input.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-bad-tasks", 0, 1, 1, 2, 4),
            essay: Some("An essay about scoring sentiment.".to_string()),
            input_schema: Some(valid_scalar_schema()),
            essay_tasks: Some("Tasks for scoring.".to_string()),
            tasks: Some(vec![
                invalid_scalar_leaf_task(),
                invalid_scalar_leaf_task(),
            ]),
            tasks_length: Some(2),
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5200);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
    });
}

#[test]
fn test_valid_schema_valid_tasks_scalar_leaf() {
    // Scalar leaf with valid input schema and pre-built tasks.
    // Note: the tasks use RichContent::Text responses (plain strings) which
    // fail CV29 validation ("compiled response must be an array of content
    // parts, not a plain string"). The invention errors out during
    // validate_initial_state. The snapshot captures this expected error.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-good-sl", 0, 1, 1, 2, 4),
            essay: None,
            input_schema: Some(valid_scalar_schema()),
            essay_tasks: Some("Good tasks incoming.".to_string()),
            tasks: Some(vec![
                valid_scalar_leaf_task(),
                valid_scalar_leaf_task(),
            ]),
            tasks_length: Some(2),
            description: Some("A valid scalar function.".to_string()),
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5300);
        let result = normalize(run_recursive_invention(&client, request).await);
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert_snapshot(
            &json,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_schema_valid_tasks_scalar_leaf.json"),
            include_str!("../../../../assets/functions/inventions/recursive_client_tests/valid_schema_valid_tasks_scalar_leaf.json"),
        );
    });
}

#[test]
fn test_valid_vector_schema_valid_tasks() {
    // Vector leaf with valid schema and tasks — should succeed.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
            params: params("inv-good-vl", 0, 1, 1, 2, 4),
            essay: Some("Ranking things.".to_string()),
            input_schema: Some(valid_vector_schema()),
            essay_tasks: None,
            tasks: Some(vec![
                valid_vector_leaf_task(),
                valid_vector_leaf_task(),
            ]),
            tasks_length: Some(2),
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5400);
        let result = normalize(run_recursive_invention(&client, request).await);
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert_snapshot(
            &json,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_vector_schema_valid_tasks.json"),
            include_str!("../../../../assets/functions/inventions/recursive_client_tests/valid_vector_schema_valid_tasks.json"),
        );
    });
}

#[test]
fn test_predicted_tasks_length_too_low() {
    // tasks_length = 0, but min_leaf_width = 2 — should fail.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-tl-low", 0, 1, 1, 2, 4),
            essay: Some("Writing an essay.".to_string()),
            input_schema: None,
            essay_tasks: Some("Tasks essay.".to_string()),
            tasks: None,
            tasks_length: Some(0),
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5500);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(
            err.contains("tasks_length") && err.contains("outside bounds"),
            "expected tasks_length bounds error, got: {err}",
        );
    });
}

#[test]
fn test_predicted_tasks_length_too_high() {
    // tasks_length = 99, but max_leaf_width = 4 — should fail.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
            params: params("inv-tl-high", 0, 1, 1, 2, 4),
            essay: None,
            input_schema: Some(valid_vector_schema()),
            essay_tasks: None,
            tasks: None,
            tasks_length: Some(99),
            description: Some("Description present.".to_string()),
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5600);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(
            err.contains("tasks_length") && err.contains("outside bounds"),
            "expected tasks_length bounds error, got: {err}",
        );
    });
}

#[test]
fn test_predicted_tasks_length_too_high_branch() {
    // Branch with tasks_length = 50, but max_branch_width = 5 — should fail.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarBranch(AlphaScalarBranchState {
            params: params("inv-tl-branch", 1, 3, 5, 2, 4),
            essay: Some("Branch essay.".to_string()),
            input_schema: Some(valid_scalar_schema()),
            essay_tasks: Some("Branch tasks essay.".to_string()),
            tasks: None,
            tasks_length: Some(50),
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5700);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(
            err.contains("tasks_length") && err.contains("outside bounds"),
            "expected tasks_length bounds error, got: {err}",
        );
    });
}

#[test]
fn test_predicted_tasks_length_below_branch_min() {
    // Vector branch with tasks_length = 1, but min_branch_width = 3.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaVectorBranch(AlphaVectorBranchState {
            params: params("inv-tl-vb-low", 1, 3, 8, 2, 4),
            essay: None,
            input_schema: None,
            essay_tasks: Some("Tasks essay for vector branch.".to_string()),
            tasks: None,
            tasks_length: Some(1),
            description: Some("Vector branch description.".to_string()),
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5800);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(
            err.contains("tasks_length") && err.contains("outside bounds"),
            "expected tasks_length bounds error, got: {err}",
        );
    });
}

#[test]
fn test_valid_schema_no_tasks_with_essay() {
    // Valid schema, essay present, no tasks — should succeed (normal invention flow).
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-schema-only", 0, 1, 1, 2, 4),
            essay: Some("A great essay about things.".to_string()),
            input_schema: Some(valid_scalar_schema()),
            essay_tasks: None,
            tasks: None,
            tasks_length: None,
            description: None,
            readme: None,
            checker_seed: None,
        });
        let request = make_request(state, 5900);
        let result = normalize(run_recursive_invention(&client, request).await);
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert_snapshot(
            &json,
            concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_schema_no_tasks_with_essay.json"),
            include_str!("../../../../assets/functions/inventions/recursive_client_tests/valid_schema_no_tasks_with_essay.json"),
        );
    });
}

#[test]
fn test_invalid_schema_with_tasks_and_description() {
    // Invalid schema + tasks + description — should fail on schema validation.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let client = make_client();
        let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("inv-full-bad", 0, 1, 1, 2, 4),
            essay: Some("An elaborate essay.".to_string()),
            input_schema: Some(invalid_scalar_schema()),
            essay_tasks: Some("Essay about tasks.".to_string()),
            tasks: Some(vec![invalid_scalar_leaf_task()]),
            tasks_length: Some(1),
            description: Some("A complete but invalid function.".to_string()),
            readme: Some("# README\nThis is invalid.".to_string()),
            checker_seed: None,
        });
        let request = make_request(state, 6000);
        let err = run_recursive_invention_err(&client, request).await;
        assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
    });
}

// test_invalid_vector_schema_with_tasks removed: same reason as
// test_invalid_vector_input_schema above.
