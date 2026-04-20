//! Tests for function execution client.

use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;

use objectiveai::functions::executions::request::{
    FunctionExecutionCreateParams, Strategy,
};
use objectiveai::functions::executions::response::unary::FunctionExecution;
use objectiveai::functions::expression::InputValue;
use objectiveai::error::StatusError;

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
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_swarm<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::swarm::RemoteSwarmBase>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_function<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::FullRemoteFunction>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_profile<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::RemoteProfile>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn get_prompt<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
    ) -> Result<Option<objectiveai::functions::inventions::prompts::RemotePrompt>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }
    async fn get_function_invention_state_file<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _path: &objectiveai::RemotePath,
        _filename: &'static str,
    ) -> Result<Option<String>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }

    async fn resolve_latest<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: &ctx::Context<ctx::DefaultContextExt, PC>,
        _kind: crate::retrieval::Kind,
        _path: &objectiveai::RemotePathCommitOptional,
    ) -> Result<Option<objectiveai::RemotePath>, objectiveai::error::ResponseError> {
        Err(objectiveai::error::ResponseError {
            code: 501,
            message: serde_json::json!("stub retrieve client should not be called"),
        })
    }
}

struct StubCompletionVotesFetcher;

#[async_trait::async_trait]
impl crate::vector::completions::completion_votes_fetcher::Fetcher<ctx::DefaultContextExt>
    for StubCompletionVotesFetcher
{
    async fn fetch<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _id: &str,
    ) -> Result<
        Option<Vec<objectiveai::vector::completions::response::Vote>>,
        objectiveai::error::ResponseError,
    > {
        Ok(None)
    }
}

struct StubCacheVoteFetcher;

#[async_trait::async_trait]
impl crate::vector::completions::cache_vote_fetcher::Fetcher<ctx::DefaultContextExt>
    for StubCacheVoteFetcher
{
    async fn fetch<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _agent: &objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote,
        _messages: &[objectiveai::agent::completions::message::Message],
        _responses: &[objectiveai::agent::completions::message::RichContent],
    ) -> Result<
        Option<objectiveai::vector::completions::response::Vote>,
        objectiveai::error::ResponseError,
    > {
        Ok(None)
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

struct StubVectorUsageHandler;

#[async_trait::async_trait]
impl crate::vector::completions::usage_handler::UsageHandler<ctx::DefaultContextExt>
    for StubVectorUsageHandler
{
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _request: Arc<objectiveai::vector::completions::request::VectorCompletionCreateParams>,
        _response: objectiveai::vector::completions::response::unary::VectorCompletion,
    ) {
    }
}

struct StubFunctionUsageHandler;

#[async_trait::async_trait]
impl super::usage_handler::UsageHandler<ctx::DefaultContextExt> for StubFunctionUsageHandler {
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _request: Arc<FunctionExecutionCreateParams>,
        _response: FunctionExecution,
    ) {
    }
}

// ---------------------------------------------------------------------------
// Client construction
// ---------------------------------------------------------------------------

type TestClient = super::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    StubAgentUsageHandler,
    StubCompletionVotesFetcher,
    StubCacheVoteFetcher,
    StubVectorUsageHandler,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubFunctionUsageHandler,
>;

fn make_client() -> Arc<TestClient> {
    let retrieve_router = Arc::new(crate::retrieval::retrieve::Router::new(
        Arc::new(StubRetrieveClient),
        Arc::new(StubRetrieveClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
    ));
    let agent_client = Arc::new(crate::agent::completions::Client::new(
        Arc::new(crate::mcp::Client::new(
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
    let vector_client = Arc::new(crate::vector::completions::Client::new(
        agent_client.clone(),
        retrieve_router.clone(),
        Arc::new(StubCompletionVotesFetcher),
        Arc::new(StubCacheVoteFetcher),
        Arc::new(StubVectorUsageHandler),
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
        agent_client,
        vector_client,
        viewer_client,
        retrieve_router,
        Arc::new(StubFunctionUsageHandler),
    ))
}

fn make_request(
    function_repo: &str,
    profile_repo: &str,
    input: InputValue,
    seed: i64,
) -> Arc<FunctionExecutionCreateParams> {
    Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: function_repo.to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: profile_repo.to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input,
        split: None,
        provider: None,
        seed: Some(seed),
        stream: None,
        continuation: None,
    })
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

fn check_id(expected: &std::cell::RefCell<Option<String>>, i: usize, id: &str) {
    let mut exp = expected.borrow_mut();
    match &*exp {
        None => *exp = Some(id.to_string()),
        Some(exp) => assert_eq!(id, exp, "chunk {i} has id {id:?}, expected {exp:?}"),
    }
}

async fn run_execution(client: &Arc<TestClient>, request: Arc<FunctionExecutionCreateParams>) -> FunctionExecution {
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let stream = client
        .clone()
        .create_streaming(ctx, request)
        .await
        .expect("create_streaming should succeed");
    let expected_created = std::cell::Cell::new(None);
    let expected_id = std::cell::RefCell::new(None);
    let agg = crate::stream_harness::consume_stream(
        Box::pin(stream),
        |agg, c| agg.push(c),
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.tasks.len() <= 1, "chunk {i} has {} tasks, expected at most 1", chunk.tasks.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.tasks.len() <= 1, "chunk {i} has {} tasks, expected at most 1", chunk.tasks.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_id(&expected_id, i, &chunk.id);
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    FunctionExecution::from(agg)
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn normalize(mut fe: FunctionExecution) -> FunctionExecution {
    fe.normalize_for_tests();
    fe
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    crate::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_EXECUTIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// mock-1: Simple scalar leaf, single task, binary classification, seed 42.
#[tokio::test]
async fn test_mock_1_scalar_leaf_binary_seed_42() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_1_scalar_leaf_binary_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_1_scalar_leaf_binary_seed_42.json"),
    );
}

/// mock-2: Multi-task scalar with skip condition (include_sentiment=false), seed 42.
#[tokio::test]
async fn test_mock_2_scalar_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "spam-with-optional-sentiment",
        "instruction-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Buy cheap watches now!!!".into()),
            "include_sentiment".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_2_scalar_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_2_scalar_skip_false_seed_42.json"),
    );
}

/// mock-2: Multi-task scalar with skip condition (include_sentiment=true), seed 42.
#[tokio::test]
async fn test_mock_2_scalar_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "spam-with-optional-sentiment",
        "instruction-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("I love this product!".into()),
            "include_sentiment".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_2_scalar_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_2_scalar_skip_true_seed_42.json"),
    );
}

/// mock-3: 5-way classification scalar, seed 42.
#[tokio::test]
async fn test_mock_3_scalar_5way_seed_42() {
    let client = make_client();
    let request = make_request(
        "five-star-rating",
        "triple-mode",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("The food was amazing".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_3_scalar_5way_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_3_scalar_5way_seed_42.json"),
    );
}

/// mock-4: Simple vector ranker with 3 items, seed 42.
#[tokio::test]
async fn test_mock_4_vector_ranker_seed_42() {
    let client = make_client();
    let request = make_request(
        "item-ranker",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Apple".into()),
                InputValue::String("Banana".into()),
                InputValue::String("Cherry".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_4_vector_ranker_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_4_vector_ranker_seed_42.json"),
    );
}

/// mock-5: Vector ranker with context and multiple tasks, seed 42.
#[tokio::test]
async fn test_mock_5_vector_context_multi_task_seed_42() {
    let client = make_client();
    let request = make_request(
        "contextual-ranker",
        "contextual-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best fruit".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Apple".into()),
                InputValue::String("Banana".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_5_vector_context_multi_task_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_5_vector_context_multi_task_seed_42.json"),
    );
}

/// mock-6: Scalar with system message and multi-part user content, seed 42.
#[tokio::test]
async fn test_mock_6_scalar_system_message_seed_42() {
    let client = make_client();
    let request = make_request(
        "email-importance",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "subject".into() => InputValue::String("Meeting tomorrow".into()),
            "body".into() => InputValue::String("Don't forget the meeting at 3pm.".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_6_scalar_system_message_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_6_scalar_system_message_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// Vector leaf with 5 tasks
// ---------------------------------------------------------------------------

/// mock-7: Vector ranker with 5 scoring criteria, seed 42.
#[tokio::test]
async fn test_mock_7_vector_5_criteria_seed_42() {
    let client = make_client();
    let request = make_request(
        "five-criteria-ranker",
        "schema-heavy-trio",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Option A".into()),
                InputValue::String("Option B".into()),
                InputValue::String("Option C".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_7_vector_5_criteria_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_7_vector_5_criteria_seed_42.json"),
    );
}

/// mock-8: Vector ranker with context, 5 tasks, skip conditions (strict=false), seed 42.
#[tokio::test]
async fn test_mock_8_vector_context_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "strict-contextual-ranker",
        "logprobs-and-tool",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best answer".into()),
                "strict".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Answer 1".into()),
                InputValue::String("Answer 2".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_8_vector_context_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_8_vector_context_skip_false_seed_42.json"),
    );
}

/// mock-8: Vector ranker with context, 5 tasks, skip conditions (strict=true), seed 42.
#[tokio::test]
async fn test_mock_8_vector_context_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "strict-contextual-ranker",
        "logprobs-and-tool",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("best answer".into()),
                "strict".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Answer 1".into()),
                InputValue::String("Answer 2".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_8_vector_context_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_8_vector_context_skip_true_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// Scalar branch functions
// ---------------------------------------------------------------------------

/// mock-9: Scalar branch combining spam + importance classifiers, seed 42.
#[tokio::test]
async fn test_mock_9_scalar_branch_2_tasks_seed_42() {
    let client = make_client();
    let request = make_request(
        "spam-importance-branch",
        "schema-logprobs-solo",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Important project update".into()),
            "subject".into() => InputValue::String("Project update".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_9_scalar_branch_2_tasks_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_9_scalar_branch_2_tasks_seed_42.json"),
    );
}

/// mock-10: Scalar branch combining binary, 5-way, importance (one agent errors), seed 42.
#[tokio::test]
async fn test_mock_10_scalar_branch_3_tasks_error_seed_42() {
    let client = make_client();
    let request = make_request(
        "triple-classifier-branch",
        "trio-with-error-agent",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Great service!".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_10_scalar_branch_3_tasks_error_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_10_scalar_branch_3_tasks_error_seed_42.json"),
    );
}

/// mock-11: Scalar branch with skip condition (include_sentiment=false), seed 42.
#[tokio::test]
async fn test_mock_11_scalar_branch_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "classifier-with-optional-sentiment",
        "tool-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Check this out".into()),
            "include_sentiment".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_11_scalar_branch_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_11_scalar_branch_skip_false_seed_42.json"),
    );
}

/// mock-11: Scalar branch with skip condition (include_sentiment=true), seed 42.
#[tokio::test]
async fn test_mock_11_scalar_branch_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "classifier-with-optional-sentiment",
        "tool-and-schema",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Check this out".into()),
            "include_sentiment".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_11_scalar_branch_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_11_scalar_branch_skip_true_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// Vector branch functions
// ---------------------------------------------------------------------------

/// mock-12: Vector branch with two vector sub-function rankers, seed 42.
#[tokio::test]
async fn test_mock_12_vector_branch_2_vector_seed_42() {
    let client = make_client();
    let request = make_request(
        "dual-ranker-branch",
        "logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Red".into()),
                InputValue::String("Blue".into()),
                InputValue::String("Green".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_12_vector_branch_2_vector_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_12_vector_branch_2_vector_seed_42.json"),
    );
}

/// mock-13: Vector branch mixing scalar and vector sub-functions, seed 42.
#[tokio::test]
async fn test_mock_13_vector_branch_mixed_seed_42() {
    let client = make_client();
    let request = make_request(
        "mixed-scalar-vector-branch",
        "schema-solo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("favorite color".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Red".into()),
                InputValue::String("Blue".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_13_vector_branch_mixed_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_13_vector_branch_mixed_seed_42.json"),
    );
}

/// mock-14: Vector branch with skip on sub-function (include_quality=false), seed 42.
#[tokio::test]
async fn test_mock_14_vector_branch_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "ranker-with-optional-quality",
        "trio-with-error-instruction",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these".into()),
                "include_quality".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("X".into()),
                InputValue::String("Y".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_14_vector_branch_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_14_vector_branch_skip_false_seed_42.json"),
    );
}

/// mock-14: Vector branch with skip on sub-function (include_quality=true), seed 42.
#[tokio::test]
async fn test_mock_14_vector_branch_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "ranker-with-optional-quality",
        "trio-with-error-instruction",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these".into()),
                "include_quality".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("X".into()),
                InputValue::String("Y".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_14_vector_branch_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_14_vector_branch_skip_true_seed_42.json"),
    );
}

/// mock-15: Vector branch with 3 vector sub-functions and high logprobs, seed 42.
#[tokio::test]
async fn test_mock_15_vector_branch_3_vector_logprobs_seed_42() {
    let client = make_client();
    let request = make_request(
        "triple-ranker-branch",
        "high-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_15_vector_branch_3_vector_logprobs_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_15_vector_branch_3_vector_logprobs_seed_42.json"),
    );
}

/// mock-16: Vector branch with 4 tasks, error agent, logprobs, seed 42.
#[tokio::test]
async fn test_mock_16_vector_branch_4_tasks_error_logprobs_seed_42() {
    let client = make_client();
    let request = make_request(
        "four-way-vector-branch",
        "quad-with-error",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Evaluate these options".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("First".into()),
                InputValue::String("Second".into()),
                InputValue::String("Third".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_16_vector_branch_4_tasks_error_logprobs_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_16_vector_branch_4_tasks_error_logprobs_seed_42.json"),
    );
}

/// mock-17: Vector branch with mixed tasks, skip conditions (deep=false), seed 42.
#[tokio::test]
async fn test_mock_17_vector_branch_mixed_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "deep-optional-mixed-branch",
        "max-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("compare these".into()),
                "deep".into() => InputValue::Boolean(false),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Foo".into()),
                InputValue::String("Bar".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_false_seed_42.json"),
    );
}

/// mock-17: Vector branch with mixed tasks, skip conditions (deep=true), seed 42.
#[tokio::test]
async fn test_mock_17_vector_branch_mixed_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "deep-optional-mixed-branch",
        "max-logprobs-duo",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("compare these".into()),
                "deep".into() => InputValue::Boolean(true),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("Foo".into()),
                InputValue::String("Bar".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_17_vector_branch_mixed_skip_true_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// Super branch tests (branch functions whose tasks are branch functions)
// ---------------------------------------------------------------------------

/// mock-18: Scalar super branch, 2 scalar branch sub-functions, seed 42.
#[tokio::test]
async fn test_mock_18_scalar_super_branch_seed_42() {
    let client = make_client();
    let request = make_request(
        "nested-scalar-super-branch",
        "expanded-nested-scalar",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
            "subject".into() => InputValue::String("greeting".into()),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_18_scalar_super_branch_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_18_scalar_super_branch_seed_42.json"),
    );
}

/// mock-19: Scalar super branch with skip (thorough=false), seed 42.
#[tokio::test]
async fn test_mock_19_scalar_super_branch_skip_false_seed_42() {
    let client = make_client();
    let request = make_request(
        "skipable-nested-scalar-branch",
        "mixed-nested-with-skip",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Test input".into()),
            "subject".into() => InputValue::String("testing".into()),
            "thorough".into() => InputValue::Boolean(false),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_false_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_false_seed_42.json"),
    );
}

/// mock-19: Scalar super branch with skip (thorough=true), seed 42.
#[tokio::test]
async fn test_mock_19_scalar_super_branch_skip_true_seed_42() {
    let client = make_client();
    let request = make_request(
        "skipable-nested-scalar-branch",
        "mixed-nested-with-skip",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Test input".into()),
            "subject".into() => InputValue::String("testing".into()),
            "thorough".into() => InputValue::Boolean(true),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_true_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_19_scalar_super_branch_skip_true_seed_42.json"),
    );
}

/// mock-20: Vector super branch, 2 vector branch sub-functions, seed 42.
#[tokio::test]
async fn test_mock_20_vector_super_branch_seed_42() {
    let client = make_client();
    let request = make_request(
        "nested-vector-super-branch",
        "nested-vector-inline-remote",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
                InputValue::String("Gamma".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_20_vector_super_branch_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_20_vector_super_branch_seed_42.json"),
    );
}

/// mock-21: Vector super branch with context, 3 vector branch sub-functions, seed 42.
#[tokio::test]
async fn test_mock_21_vector_super_branch_context_seed_42() {
    let client = make_client();
    let request = make_request(
        "contextual-nested-vector-branch",
        "deep-nested-vector",
        InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("rank these options".into()),
            }),
            "items".into() => InputValue::Array(vec![
                InputValue::String("One".into()),
                InputValue::String("Two".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_21_vector_super_branch_context_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_21_vector_super_branch_context_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// Placeholder function tasks
// ---------------------------------------------------------------------------

/// Inline scalar function with only placeholder tasks, inline auto profile.
#[tokio::test]
async fn test_inline_scalar_placeholder_seed_42() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Inline(
            objectiveai::functions::FullInlineFunction::Standard(
                objectiveai::functions::InlineFunction::Scalar {
                    tasks: vec![
                        objectiveai::functions::TaskExpression::PlaceholderScalarFunction(
                            objectiveai::functions::PlaceholderScalarFunctionTaskExpression {
                                input_schema: serde_json::from_value(serde_json::json!({
                                    "type": "object",
                                    "properties": { "text": { "type": "string" } },
                                    "required": ["text"]
                                })).unwrap(),
                                skip: None,
                                map: None,
                                input: objectiveai::functions::expression::WithExpression::Expression(
                                    objectiveai::functions::expression::Expression::Starlark(
                                        "{'text': input['text']}".to_string(),
                                    ),
                                ),
                                output: objectiveai::functions::expression::Expression::Special(
                                    objectiveai::functions::expression::Special::Output,
                                ),
                            },
                        ),
                        objectiveai::functions::TaskExpression::PlaceholderScalarFunction(
                            objectiveai::functions::PlaceholderScalarFunctionTaskExpression {
                                input_schema: serde_json::from_value(serde_json::json!({
                                    "type": "object",
                                    "properties": { "text": { "type": "string" } },
                                    "required": ["text"]
                                })).unwrap(),
                                skip: None,
                                map: None,
                                input: objectiveai::functions::expression::WithExpression::Expression(
                                    objectiveai::functions::expression::Expression::Starlark(
                                        "{'text': input['text']}".to_string(),
                                    ),
                                ),
                                output: objectiveai::functions::expression::Expression::Special(
                                    objectiveai::functions::expression::Special::Output,
                                ),
                            },
                        ),
                    ],
                },
            ),
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Inline(
            objectiveai::functions::InlineProfile::Auto(
                objectiveai::swarm::InlineSwarmBase {
                    agents: vec![
                        objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
                            count: 1,
                            inner: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
                                objectiveai::agent::InlineAgentBaseWithFallbacks {
                                    inner: objectiveai::agent::InlineAgentBase::Mock(
                                        objectiveai::agent::mock::AgentBase {
                                            upstream: objectiveai::agent::mock::Upstream::Mock,
                                            output_mode: objectiveai::agent::mock::OutputMode::Instruction,
                                            top_logprobs: None,
                                            error: None,
                                            error_probability: None,
                                            mode: None,
                                            mcp_servers: None,
                                        },
                                    ),
                                    fallbacks: None,
                                },
                            ),
                        },
                    ],
                    weights: Some(objectiveai::Weights::Weights(vec![Decimal::ONE])),
                },
            ),
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        split: None,
        provider: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/inline_scalar_placeholder_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/inline_scalar_placeholder_seed_42.json"),
    );
}

/// mock-25: Remote scalar function with only placeholder tasks, remote swarm profile.
#[tokio::test]
async fn test_mock_25_scalar_placeholder_remote_swarm_seed_42() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "dual-placeholder".to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "schema-and-tool".to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("Hello world".into()),
        }),
        split: None,
        provider: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_25_scalar_placeholder_remote_swarm_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_25_scalar_placeholder_remote_swarm_seed_42.json"),
    );
}

// ---------------------------------------------------------------------------
// SwissSystem strategy tests
// ---------------------------------------------------------------------------

/// mock-4: Vector ranker with 20 items, SwissSystem with default pool/rounds, seed 7.
#[tokio::test]
async fn test_mock_4_vector_swiss_default_20_items_seed_7() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "item-ranker".to_string() },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "solo-instruction".to_string() },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: Some(Strategy::SwissSystem { pool: None, rounds: None }),
        input: InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        split: None,
        provider: None,
        seed: Some(7),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_4_vector_swiss_default_20_items_seed_7.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_4_vector_swiss_default_20_items_seed_7.json"),
    );
}

/// mock-5: Vector ranker with context and 20 items, SwissSystem pool=5 rounds=3, seed 7.
#[tokio::test]
async fn test_mock_5_vector_swiss_pool5_rounds3_20_items_seed_7() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "contextual-ranker".to_string() },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "contextual-duo".to_string() },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: Some(Strategy::SwissSystem { pool: Some(5), rounds: Some(3) }),
        input: InputValue::Object(indexmap::indexmap! {
            "context".into() => InputValue::Object(indexmap::indexmap! {
                "query".into() => InputValue::String("rank these items".into()),
            }),
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        split: None,
        provider: None,
        seed: Some(7),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_5_vector_swiss_pool5_rounds3_20_items_seed_7.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_5_vector_swiss_pool5_rounds3_20_items_seed_7.json"),
    );
}

/// mock-7: Vector ranker with 5 criteria and 20 items, SwissSystem pool=4 rounds=3, seed 7.
#[tokio::test]
async fn test_mock_7_vector_swiss_pool4_rounds3_20_items_seed_7() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "five-criteria-ranker".to_string() },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock { name: "schema-heavy-trio".to_string() },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: Some(Strategy::SwissSystem { pool: Some(4), rounds: Some(3) }),
        input: InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array((0..20).map(|i| InputValue::String(format!("Item{i}"))).collect()),
        }),
        split: None,
        provider: None,
        seed: Some(7),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_7_vector_swiss_pool4_rounds3_20_items_seed_7.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_7_vector_swiss_pool4_rounds3_20_items_seed_7.json"),
    );
}

// ---------------------------------------------------------------------------
// Mapped function tasks (MapFunction) — exercises task_index_len for mapped branches
// ---------------------------------------------------------------------------

/// mock-22: Scalar mapped branch (2 items) + 2 VCs, seed 42.
#[tokio::test]
async fn test_mock_22_scalar_mapped_branch_2_items_seed_42() {
    let client = make_client();
    let request = make_request(
        "mapped-branch-with-votes",
        "remote-swarm-mapped-branch",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_42.json"),
    );
}

/// mock-22: Scalar mapped branch (2 items) + 2 VCs, seed 123.
#[tokio::test]
async fn test_mock_22_scalar_mapped_branch_2_items_seed_123() {
    let client = make_client();
    let request = make_request(
        "mapped-branch-with-votes",
        "remote-swarm-mapped-branch",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        123,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_123.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_22_scalar_mapped_branch_2_items_seed_123.json"),
    );
}

/// mock-23: Scalar mapped branch (3 items) + 3 VCs, seed 42.
#[tokio::test]
async fn test_mock_23_scalar_mapped_branch_3_items_seed_42() {
    let client = make_client();
    let request = make_request(
        "mapped-branch-with-classifiers",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
                InputValue::String("C".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_3_items_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_3_items_seed_42.json"),
    );
}

/// mock-23: Scalar mapped branch (2 items) + 3 VCs, seed 42.
#[tokio::test]
async fn test_mock_23_scalar_mapped_branch_2_items_seed_42() {
    let client = make_client();
    let request = make_request(
        "mapped-branch-with-classifiers",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_2_items_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_23_scalar_mapped_branch_2_items_seed_42.json"),
    );
}

/// mock-24: Scalar mapped branch (2 items) + function task + 2 VCs, seed 42.
#[tokio::test]
async fn test_mock_24_scalar_mapped_branch_with_func_2_items_seed_42() {
    let client = make_client();
    let request = make_request(
        "mapped-branch-mixed-tasks",
        "remote-swarm-classifiers",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("Alpha".into()),
                InputValue::String("Beta".into()),
            ]),
        }),
        42,
    );
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/mock_24_scalar_mapped_branch_with_func_2_items_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/mock_24_scalar_mapped_branch_with_func_2_items_seed_42.json"),
    );
}

// ===========================================================================
// Error tests
// ===========================================================================

/// Helper: create a request with custom fields.
fn make_request_with_overrides(
    function_repo: &str,
    profile_repo: &str,
    overrides: impl FnOnce(&mut FunctionExecutionCreateParams),
) -> Arc<FunctionExecutionCreateParams> {
    let mut params = FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: function_repo.to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: profile_repo.to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Object(indexmap::indexmap! {}),
        split: None,
        provider: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    };
    overrides(&mut params);
    Arc::new(params)
}

/// Helper: expect create_streaming to return Err with a specific status code.
async fn expect_err(client: &Arc<TestClient>, request: Arc<FunctionExecutionCreateParams>, expected_status: u16) -> super::Error {
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    match client.clone().create_streaming(ctx, request).await {
        Ok(_) => panic!("expected create_streaming to fail, but it succeeded"),
        Err(err) => {
            assert_eq!(err.status(), expected_status, "error: {err}");
            err
        }
    }
}

/// Helper: run execution and return the aggregated result (for tests where
/// the stream succeeds but the response contains error fields).
async fn run_execution_allow_error(client: &Arc<TestClient>, request: Arc<FunctionExecutionCreateParams>) -> FunctionExecution {
    run_execution(client, request).await
}

// ---------------------------------------------------------------------------
// 1. Pre-Execution Errors
// ---------------------------------------------------------------------------

/// 1.1: InvalidRetryToken — garbage retry_token string.
#[tokio::test]
async fn test_error_1_1_invalid_retry_token() {
    let client = make_client();
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.retry_token = Some("not-a-valid-retry-token!!!".to_string());
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidRetryToken), "expected InvalidRetryToken, got: {err}");
}

/// 1.3: InvalidFunctionForStrategy — scalar function with Swiss strategy.
#[tokio::test]
async fn test_error_1_3_scalar_function_swiss_strategy() {
    let client = make_client();
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.strategy = Some(Strategy::SwissSystem { pool: None, rounds: None });
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidFunctionForStrategy(_)), "expected InvalidFunctionForStrategy, got: {err}");
}

/// 1.4: InvalidStrategy — Swiss strategy with pool=1.
#[tokio::test]
async fn test_error_1_4_invalid_strategy_pool() {
    let client = make_client();
    let request = make_request_with_overrides(
        "item-ranker",
        "solo-instruction",
        |p| {
            p.strategy = Some(Strategy::SwissSystem { pool: Some(1), rounds: Some(3) });
            p.input = InputValue::Object(indexmap::indexmap! {
                "items".into() => InputValue::Array(vec![
                    InputValue::String("A".into()),
                    InputValue::String("B".into()),
                    InputValue::String("C".into()),
                ]),
            });
        },
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidStrategy(_)), "expected InvalidStrategy, got: {err}");
}

// ---------------------------------------------------------------------------
// 2. Flat Task Profile Fetch Errors
// ---------------------------------------------------------------------------

/// 2.1: FunctionNotFound — non-existent mock function repository.
#[tokio::test]
async fn test_error_2_1_function_not_found() {
    let client = make_client();
    let request = make_request("mock-nonexistent", "solo-instruction", InputValue::Object(indexmap::indexmap! {}), 42);
    let err = expect_err(&client, request, 404).await;
    assert!(matches!(err, super::Error::FetchFunction(_)), "expected FetchFunction, got: {err}");
}

/// 2.3: ProfileNotFound — non-existent mock profile repository.
#[tokio::test]
async fn test_error_2_3_profile_not_found() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "mock-nonexistent",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 404).await;
    assert!(matches!(err, super::Error::FetchProfile(_)), "expected FetchProfile, got: {err}");
}

/// 2.5: InputSchemaMismatch — wrong input shape for mock-1.
#[tokio::test]
async fn test_error_2_5_input_schema_mismatch() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "solo-instruction",
        InputValue::Object(indexmap::indexmap! {
            "wrong_field".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InputSchemaMismatch), "expected InputSchemaMismatch, got: {err}");
}

/// 2.6: InvalidProfile — tasks length mismatch (2 task profiles for 1-task function).
#[tokio::test]
async fn test_error_2_6_tasks_length_mismatch() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "two-task-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidProfile(_)), "expected InvalidProfile, got: {err}");
}

/// 2.7: InvalidProfile — weights length mismatch (2 weights for 1-task function).
#[tokio::test]
async fn test_error_2_7_weights_length_mismatch() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "error-weights-length-mismatch",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidProfile(_)), "expected InvalidProfile, got: {err}");
}

/// 2.8: InvalidProfile — placeholder for function task.
#[tokio::test]
async fn test_error_2_8_placeholder_for_function_task() {
    let client = make_client();
    let request = make_request(
        "spam-importance-branch",
        "placeholder-and-remote-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
            "subject".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidProfile(_)), "expected InvalidProfile, got: {err}");
}

// 2.9: Removed — Remote profile for VC task is now supported (resolves via swarm fallback).

/// 2.17: InvalidAppExpression — task expression references missing key.
#[tokio::test]
async fn test_error_2_17_bad_task_expression() {
    let client = make_client();
    let request = make_request(
        "error-missing-input-key",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidAppExpression(_)), "expected InvalidAppExpression, got: {err}");
}

// 2.19: FetchSwarm — removed. Remote swarm references within profiles no longer exist;
// swarm is always inline on the profile (RemoteSwarmBase).

/// 2.20: InvalidSwarm — 1 agent but 2 profile weights.
#[tokio::test]
async fn test_error_2_20_invalid_swarm() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "error-weight-count-mismatch",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InvalidSwarm(_)), "expected InvalidSwarm, got: {err}");
}

/// 2.21: Recursive FunctionNotFound — branch references mock-999.
#[tokio::test]
async fn test_error_2_21_recursive_function_not_found() {
    let client = make_client();
    let request = make_request(
        "error-missing-sub-function",
        "baseline-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 404).await;
    assert!(matches!(err, super::Error::FetchFunction(_)), "expected FetchFunction, got: {err}");
}

/// 2.22: Recursive ProfileNotFound — tasks profile references mock-999.
#[tokio::test]
async fn test_error_2_22_recursive_profile_not_found() {
    let client = make_client();
    let request = make_request(
        "spam-importance-branch",
        "dangling-and-valid-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
            "subject".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 404).await;
    assert!(matches!(err, super::Error::FetchProfile(_)), "expected FetchProfile, got: {err}");
}

/// 2.23: CircularDependency — simple cycle A→B→A.
#[tokio::test]
async fn test_error_2_23_circular_dependency_simple() {
    let client = make_client();
    let request = make_request(
        "error-cycle-a",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::CircularDependency(_)), "expected CircularDependency, got: {err}");
}

/// 2.24: CircularDependency — complex cycle A→{B,C}, B→C, C→B.
#[tokio::test]
async fn test_error_2_24_circular_dependency_complex() {
    let client = make_client();
    let request = make_request(
        "error-cycle-abc-a",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::CircularDependency(_)), "expected CircularDependency, got: {err}");
}

/// 2.25: Recursive InputSchemaMismatch — wrong input for sub-function.
#[tokio::test]
async fn test_error_2_25_recursive_input_schema_mismatch() {
    let client = make_client();
    let request = make_request(
        "error-wrong-sub-input",
        "baseline-tasks",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let err = expect_err(&client, request, 400).await;
    assert!(matches!(err, super::Error::InputSchemaMismatch), "expected InputSchemaMismatch, got: {err}");
}

// ---------------------------------------------------------------------------
// 3. Vector Completion Errors (execution-time)
// ---------------------------------------------------------------------------

/// 3.1: All agents error — VC agents fail, completions have error finish_reason,
/// output is fallback uniform → weighted sum to 0.5.
#[tokio::test]
async fn test_error_3_1_all_agents_error() {
    let client = make_client();
    let request = make_request(
        "binary-classifier",
        "error-all-agents-fail",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert_eq!(result.tasks.len(), 1);
    match &result.tasks[0] {
        objectiveai::functions::executions::response::unary::Task::VectorCompletion(vt) => {
            // The task itself should not have an error (VC "succeeds" with fallback).
            assert!(vt.error.is_none(), "expected no task-level error, got: {:?}", vt.error);
            assert!(!vt.inner.completions.is_empty(), "expected at least one completion");
            for completion in &vt.inner.completions {
                // Each agent completion should have an error set.
                assert!(
                    completion.inner.error.is_some(),
                    "expected error on agent completion, got None",
                );
            }
        }
        other => panic!("expected VectorCompletion task, got: {other:?}"),
    }
    // Output is the fallback weighted sum of uniform distribution.
    assert!(
        matches!(&result.output.output, objectiveai::functions::expression::TaskOutputOwned::Scalar(s) if *s == rust_decimal::dec!(0.5)),
        "expected Scalar(0.5) fallback, got: {:?}",
        result.output,
    );
}

// ---------------------------------------------------------------------------
// 4. Task Output Expression Errors (execution-time)
// ---------------------------------------------------------------------------

/// 4.1: Output expression evaluation fails (references nonexistent field).
#[tokio::test]
async fn test_error_4_1_output_expression_fails() {
    let client = make_client();
    let request = make_request(
        "error-bad-output-field",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.2: Scalar output out of range (returns -1.0).
#[tokio::test]
async fn test_error_4_2_scalar_output_out_of_range() {
    let client = make_client();
    let request = make_request(
        "error-scalar-out-of-range",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.3: Scalar function got vector output.
#[tokio::test]
async fn test_error_4_3_scalar_got_vector() {
    let client = make_client();
    let request = make_request(
        "error-scalar-returns-vector",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.4: Vector output bad sum (scores doubled).
#[tokio::test]
async fn test_error_4_4_vector_output_bad_sum() {
    let client = make_client();
    let request = make_request(
        "error-vector-bad-sum",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.5: Vector function got scalar output.
#[tokio::test]
async fn test_error_4_5_vector_got_scalar() {
    let client = make_client();
    let request = make_request(
        "error-vector-returns-scalar",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "items".into() => InputValue::Array(vec![
                InputValue::String("A".into()),
                InputValue::String("B".into()),
            ]),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.6: Output returns nested list (Vectors variant).
#[tokio::test]
async fn test_error_4_6_output_vectors_variant() {
    let client = make_client();
    let request = make_request(
        "error-nested-list-output",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

/// 4.7: Output expression returns None (Err value).
#[tokio::test]
async fn test_error_4_7_output_returns_none() {
    let client = make_client();
    let request = make_request(
        "error-none-output",
        "baseline-auto",
        InputValue::Object(indexmap::indexmap! {
            "text".into() => InputValue::String("test".into()),
        }),
        42,
    );
    let result = run_execution_allow_error(&client, request).await;
    assert!(result.error.is_some(), "expected error on response");
    assert!(
        matches!(result.output.output, objectiveai::functions::expression::TaskOutputOwned::Err(_)),
        "expected Err output, got: {:?}",
        result.output,
    );
}

// ---------------------------------------------------------------------------
// 6. Reasoning Errors
// ---------------------------------------------------------------------------

/// 6.1: Reasoning agent error — mock agent with error=true.
#[tokio::test]
async fn test_error_6_1_reasoning_agent_error() {
    let client = make_client();
    let request = make_request_with_overrides(
        "binary-classifier",
        "solo-instruction",
        |p| {
            p.reasoning = Some(objectiveai::functions::executions::request::Reasoning {
                agent: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional::AgentBase(
                    objectiveai::agent::InlineAgentBaseWithFallbacks {
                        inner: objectiveai::agent::InlineAgentBase::Mock(objectiveai::agent::mock::AgentBase {
                            upstream: objectiveai::agent::mock::Upstream::Mock,
                            output_mode: objectiveai::agent::mock::OutputMode::Instruction,
                            top_logprobs: None,
                            error: Some(true),
                            error_probability: None,
                            mode: None,
                            mcp_servers: None,
                        }),
                        fallbacks: None,
                    },
                ),
            });
            p.input = InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("test".into()),
            });
        },
    );
    // The stream succeeds but the reasoning chunk will have an error.
    let result = run_execution_allow_error(&client, request).await;
    // The execution itself should succeed (output is valid).
    // The reasoning should have an error.
    assert!(
        result.reasoning.as_ref().is_some_and(|r| r.error.is_some()),
        "expected reasoning error, got: {:?}",
        result.reasoning,
    );
}

// ===========================================================================
// Split tests
// ===========================================================================

/// Split: run scalar binary-classifier on 3 inputs, expect Vector output.
#[tokio::test]
async fn test_split_scalar_binary_seed_42() {
    let client = make_client();
    let request = Arc::new(FunctionExecutionCreateParams {
        function: objectiveai::functions::FullInlineFunctionOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "binary-classifier".to_string(),
            },
        ),
        profile: objectiveai::functions::InlineProfileOrRemoteCommitOptional::Remote(
            objectiveai::RemotePathCommitOptional::Mock {
                name: "solo-instruction".to_string(),
            },
        ),
        retry_token: None,
        from_cache: None,
        reasoning: None,
        strategy: None,
        input: InputValue::Array(vec![
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Hello world".into()),
            }),
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Buy cheap watches".into()),
            }),
            InputValue::Object(indexmap::indexmap! {
                "text".into() => InputValue::String("Good morning".into()),
            }),
        ]),
        split: Some(true),
        provider: None,
        seed: Some(42),
        stream: None,
        continuation: None,
    });
    let result = normalize(run_execution(&client, request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/executions/client_tests/split_scalar_binary_seed_42.json"),
        include_str!("../../../assets/functions/executions/client_tests/split_scalar_binary_seed_42.json"),
    );
}
