//! Tests for function invention client.

use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;

use objectiveai::functions::expression::{
    InputSchema, ObjectInputSchema, ArrayInputSchema, AnyOfInputSchema,
    StringInputSchema, IntegerInputSchema, NumberInputSchema,
    BooleanInputSchema, ImageInputSchema, AudioInputSchema,
    VideoInputSchema, FileInputSchema,
};
use objectiveai::functions::inventions::request::FunctionInventionCreateParams;
use objectiveai::functions::inventions::response::unary::FunctionInvention;
use objectiveai::functions::inventions::state::{Params, ParamsState};
use objectiveai::functions::inventions::state::{
    AlphaScalarLeafState, AlphaScalarBranchState,
    AlphaVectorLeafState, AlphaVectorBranchState,
};
use objectiveai::functions::alpha_vector::expression::VectorFunctionInputSchema;

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
impl super::usage_handler::UsageHandler<ctx::DefaultContextExt> for StubInventionUsageHandler {
    async fn handle_usage<PC: crate::ctx::persistent_cache::PersistentCacheClient>(
        &self,
        _ctx: ctx::Context<ctx::DefaultContextExt, PC>,
        _request: Arc<FunctionInventionCreateParams>,
        _response: FunctionInvention,
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
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubAgentUsageHandler,
    StubInventionUsageHandler,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
    crate::retrieval::retrieve::mock::MockClient,
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
        std::path::PathBuf::from("/tmp/objectiveai-test"),
        "ObjectiveAI".to_string(),
        "noreply@objectiveai.dev".to_string(),
    ));
    let function_retrieve_router = Arc::new(crate::retrieval::retrieve::Router::new(
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
    ));
    Arc::new(super::Client::new(
        agent_client,
        github_client,
        filesystem_client,
        function_retrieve_router,
        Arc::new(StubInventionUsageHandler),
        true,
        false,
    ))
}

fn make_request(state: ParamsState, seed: i64) -> Arc<FunctionInventionCreateParams> {
    Arc::new(FunctionInventionCreateParams {
        remote: None,
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
        spec: "Test function spec for mock invention.".to_string(),
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

async fn run_invention(
    client: &Arc<TestClient>,
    request: Arc<FunctionInventionCreateParams>,
) -> FunctionInvention {
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let stream = client
        .clone()
        .create_streaming(ctx, request)
        .await
        .expect("create_streaming should succeed");
    let expected_created = std::cell::Cell::new(None);
    let agg = crate::stream_harness::consume_stream_acc(
        Box::pin(stream),
        |agg, c| agg.push(c),
        |chunk, errors: &mut Vec<objectiveai::error::ResponseError>| {
            if let Some(e) = &chunk.error {
                errors.push(e.clone());
            }
            for completion in &chunk.completions {
                if let Some(e) = &completion.inner.error {
                    errors.push(e.clone());
                }
            }
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.function.is_none(), "chunk {i} (non-final) has function, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
            assert!(chunk.function.is_none(), "chunk {i} (non-final) has function, expected None");
        },
        |i, chunk, errors| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
            assert!(chunk.function.is_some(), "final chunk {i} has no function, expected Some. errors: {}", serde_json::to_string(errors).unwrap());
            assert!(chunk.state.is_some(), "final chunk {i} has no state, expected Some");
        },
        Vec::new(),
    ).await;
    FunctionInvention::from(agg)
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn normalize(mut fi: FunctionInvention) -> FunctionInvention {
    fi.normalize_for_tests();
    fi
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    crate::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_FUNCTIONS_INVENTIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Test macro
// ---------------------------------------------------------------------------

/// Generates 10 snapshot tests (seeds 0–9), all under a module named
/// `$test_name`. `$base` is the snapshot filename base (e.g. `"scalar_leaf_s42"`),
/// producing files `scalar_leaf_s42_0.json` through `scalar_leaf_s42_9.json`.
macro_rules! invention_test_10x {
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
                    let result = normalize(run_invention(&client, request).await);
                    assert!(result.function.is_some(), "seed {seed}: function should be built. error: {:?}", result.error);
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    assert_snapshot(&json, path, expected);
                });
            }

            #[test]
            fn seed_0() {
                run_snapshot(
                    0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_0.json")),
                );
            }

            #[test]
            fn seed_1() {
                run_snapshot(
                    1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_1.json")),
                );
            }

            #[test]
            fn seed_2() {
                run_snapshot(
                    2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_2.json")),
                );
            }

            #[test]
            fn seed_3() {
                run_snapshot(
                    3,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_3.json")),
                );
            }

            #[test]
            fn seed_4() {
                run_snapshot(
                    4,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_4.json")),
                );
            }

            #[test]
            fn seed_5() {
                run_snapshot(
                    5,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_5.json")),
                );
            }

            #[test]
            fn seed_6() {
                run_snapshot(
                    6,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_6.json")),
                );
            }

            #[test]
            fn seed_7() {
                run_snapshot(
                    7,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_7.json")),
                );
            }

            #[test]
            fn seed_8() {
                run_snapshot(
                    8,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_8.json")),
                );
            }

            #[test]
            fn seed_9() {
                run_snapshot(
                    9,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_9.json")),
                );
            }
        }
    };
}

/// Same as `invention_test_10x!` but with a pre-provided input schema.
macro_rules! invention_test_10x_schema {
    (
        $test_name:ident,
        $variant:ident, $state_ty:ident,
        $name:expr, $depth:expr,
        $min_b:expr, $max_b:expr, $min_l:expr, $max_l:expr,
        $base_seed:expr,
        $base:expr,
        $schema:expr
    ) => {
        mod $test_name {
            use super::*;

            fn make_state(seed_offset: i64) -> (ParamsState, i64) {
                (
                    ParamsState::$variant($state_ty {
                        params: params($name, $depth, $min_b, $max_b, $min_l, $max_l),
                        essay: None,
                        input_schema: Some($schema),
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
                    let result = normalize(run_invention(&client, request).await);
                    assert!(result.function.is_some(), "seed {seed}: function should be built. error: {:?}", result.error);
                    let json = serde_json::to_string_pretty(&result).unwrap();
                    assert_snapshot(&json, path, expected);
                });
            }

            #[test]
            fn seed_0() {
                run_snapshot(
                    0,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_0.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_0.json")),
                );
            }

            #[test]
            fn seed_1() {
                run_snapshot(
                    1,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_1.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_1.json")),
                );
            }

            #[test]
            fn seed_2() {
                run_snapshot(
                    2,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_2.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_2.json")),
                );
            }

            #[test]
            fn seed_3() {
                run_snapshot(
                    3,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_3.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_3.json")),
                );
            }

            #[test]
            fn seed_4() {
                run_snapshot(
                    4,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_4.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_4.json")),
                );
            }

            #[test]
            fn seed_5() {
                run_snapshot(
                    5,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_5.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_5.json")),
                );
            }

            #[test]
            fn seed_6() {
                run_snapshot(
                    6,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_6.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_6.json")),
                );
            }

            #[test]
            fn seed_7() {
                run_snapshot(
                    7,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_7.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_7.json")),
                );
            }

            #[test]
            fn seed_8() {
                run_snapshot(
                    8,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_8.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_8.json")),
                );
            }

            #[test]
            fn seed_9() {
                run_snapshot(
                    9,
                    concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/client_tests/", $base, "_9.json"),
                    include_str!(concat!("../../../assets/functions/inventions/client_tests/", $base, "_9.json")),
                );
            }
        }
    };
}

// ---------------------------------------------------------------------------
// Schema helpers — build wacky input schemas for pre-provided tests
// ---------------------------------------------------------------------------

fn obj(props: Vec<(&str, InputSchema)>, required: Vec<&str>) -> ObjectInputSchema {
    ObjectInputSchema {
        r#type: Default::default(),
        description: None,
        properties: props.into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
        required: Some(required.into_iter().map(String::from).collect()),
    }
}

fn arr(items: InputSchema, min: u64, max: u64) -> InputSchema {
    InputSchema::Array(ArrayInputSchema {
        r#type: Default::default(),
        description: None,
        items: Box::new(items),
        min_items: Some(min),
        max_items: Some(max),
    })
}

fn any_of(schemas: Vec<InputSchema>) -> InputSchema {
    InputSchema::AnyOf(AnyOfInputSchema { any_of: schemas })
}

fn string() -> InputSchema { InputSchema::String(StringInputSchema { r#type: Default::default(), description: None, r#enum: None }) }
fn integer() -> InputSchema { InputSchema::Integer(IntegerInputSchema { r#type: Default::default(), description: None, minimum: None, maximum: None }) }
fn number() -> InputSchema { InputSchema::Number(NumberInputSchema { r#type: Default::default(), description: None, minimum: None, maximum: None }) }
fn boolean() -> InputSchema { InputSchema::Boolean(BooleanInputSchema { r#type: Default::default(), description: None }) }
fn image() -> InputSchema { InputSchema::Image(ImageInputSchema { r#type: Default::default(), description: None }) }
fn audio() -> InputSchema { InputSchema::Audio(AudioInputSchema { r#type: Default::default(), description: None }) }
fn video() -> InputSchema { InputSchema::Video(VideoInputSchema { r#type: Default::default(), description: None }) }
fn file() -> InputSchema { InputSchema::File(FileInputSchema { r#type: Default::default(), description: None }) }
fn nested_obj(props: Vec<(&str, InputSchema)>, required: Vec<&str>) -> InputSchema {
    InputSchema::Object(obj(props, required))
}

/// Scalar schema 1: object with anyOf property (string | image | nested object)
fn scalar_schema_anyof_chaos() -> ObjectInputSchema {
    obj(
        vec![
            ("payload", any_of(vec![
                string(),
                image(),
                nested_obj(vec![
                    ("caption", string()),
                    ("score", number()),
                ], vec!["caption"]),
            ])),
            ("tags", arr(string(), 1, 10)),
            ("priority", integer()),
        ],
        vec!["payload", "tags", "priority"],
    )
}

/// Scalar schema 2: deeply nested objects with media at leaves
fn scalar_schema_deep_media() -> ObjectInputSchema {
    obj(
        vec![
            ("submission", nested_obj(vec![
                ("content", nested_obj(vec![
                    ("body", nested_obj(vec![
                        ("text", string()),
                        ("attachment", any_of(vec![image(), audio(), video(), file()])),
                    ], vec!["text", "attachment"])),
                    ("metadata", nested_obj(vec![
                        ("author", string()),
                        ("timestamp", integer()),
                    ], vec!["author"])),
                ], vec!["body"])),
            ], vec!["content"])),
            ("verified", boolean()),
        ],
        vec!["submission", "verified"],
    )
}

/// Scalar schema 3: array of anyOf(object, string) with nested arrays
fn scalar_schema_array_madness() -> ObjectInputSchema {
    obj(
        vec![
            ("entries", arr(
                any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("label", string()),
                        ("values", arr(any_of(vec![number(), integer(), boolean()]), 1, 5)),
                        ("photo", image()),
                    ], vec!["label", "values", "photo"]),
                ]),
                2, 20,
            )),
            ("query", string()),
        ],
        vec!["entries", "query"],
    )
}

/// Scalar schema 4: wide object with every type + anyOf array
fn scalar_schema_kitchen_sink() -> ObjectInputSchema {
    obj(
        vec![
            ("name", string()),
            ("age", integer()),
            ("score", number()),
            ("active", boolean()),
            ("avatar", image()),
            ("voicemail", audio()),
            ("demo", video()),
            ("resume", file()),
            ("aliases", arr(any_of(vec![string(), integer()]), 1, 8)),
            ("extra", any_of(vec![
                string(),
                arr(nested_obj(vec![
                    ("key", string()),
                    ("val", any_of(vec![number(), boolean(), image()])),
                ], vec!["key", "val"]), 1, 3),
            ])),
        ],
        vec!["name", "age", "score", "active", "avatar", "voicemail", "demo", "resume", "aliases", "extra"],
    )
}

/// Vector schema 1: items are anyOf(image, object with nested anyOf), context has arrays
fn vector_schema_multimedia_ranking() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("criteria", arr(string(), 1, 5)),
                ("reference_image", image()),
            ],
            vec!["criteria", "reference_image"],
        )),
        items: arr(
            any_of(vec![
                image(),
                nested_obj(vec![
                    ("visual", image()),
                    ("caption", string()),
                    ("metadata", any_of(vec![
                        string(),
                        nested_obj(vec![
                            ("source", string()),
                            ("confidence", number()),
                        ], vec!["source", "confidence"]),
                    ])),
                ], vec!["visual", "caption"]),
            ]),
            2, 6,
        ),
    }
}

/// Vector schema 2: items are objects with deeply nested anyOf arrays
fn vector_schema_nested_chaos() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("prompt", string()),
                ("settings", nested_obj(vec![
                    ("temperature", number()),
                    ("mode", any_of(vec![string(), integer()])),
                ], vec!["temperature"])),
            ],
            vec!["prompt"],
        )),
        items: arr(
            nested_obj(vec![
                ("content", any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("parts", arr(any_of(vec![string(), image(), audio()]), 1, 2)),
                        ("format", string()),
                    ], vec!["parts", "format"]),
                ])),
                ("scores", arr(number(), 1, 2)),
            ], vec!["content", "scores"]),
            2, 4,
        ),
    }
}

/// Vector schema 3: simple string items, complex context with nested objects
fn vector_schema_rich_context() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: Some(obj(
            vec![
                ("rubric", nested_obj(vec![
                    ("dimensions", arr(
                        nested_obj(vec![
                            ("name", string()),
                            ("weight", number()),
                            ("examples", arr(any_of(vec![string(), image()]), 1, 2)),
                        ], vec!["name", "weight"]),
                        1, 2,
                    )),
                    ("passing_threshold", number()),
                ], vec!["dimensions", "passing_threshold"])),
                ("evaluator", any_of(vec![
                    string(),
                    nested_obj(vec![
                        ("id", integer()),
                        ("credentials", arr(string(), 0, 2)),
                    ], vec!["id"]),
                ])),
            ],
            vec!["rubric"],
        )),
        items: arr(string(), 2, 4),
    }
}

/// Vector schema 4: no context, items with nested file/image/video anyOf
fn vector_schema_no_context_deep_items() -> VectorFunctionInputSchema {
    VectorFunctionInputSchema {
        context: None,
        items: arr(
            nested_obj(vec![
                ("candidate", any_of(vec![
                    file(),
                    nested_obj(vec![
                        ("document", file()),
                        ("preview", any_of(vec![image(), video()])),
                    ], vec!["document"]),
                ])),
                ("relevance_hint", any_of(vec![string(), number()])),
            ], vec!["candidate"]),
            2, 3,
        ),
    }
}


// ---------------------------------------------------------------------------
// Scalar Leaf tests (depth=0)
// ---------------------------------------------------------------------------

// Default widths (3-5), baseline
invention_test_10x!(test_scalar_leaf_s42,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-default", 0, 3, 5, 3, 5, 42,
    "scalar_leaf_s42");

// Minimum width: exactly 1 task
invention_test_10x!(test_scalar_leaf_s7,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-min-1", 0, 1, 1, 1, 1, 7,
    "scalar_leaf_s7");

// Narrow range: 2-3
invention_test_10x!(test_scalar_leaf_s1337,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-narrow", 0, 2, 3, 2, 3, 1337,
    "scalar_leaf_s1337");

// Large width: 10 tasks
invention_test_10x!(test_scalar_leaf_s999,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-wide-10", 0, 10, 10, 10, 10, 999,
    "scalar_leaf_s999");

// Asymmetric: narrow branch, wide leaf
invention_test_10x!(test_scalar_leaf_s314,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-asym", 0, 1, 2, 7, 10, 314,
    "scalar_leaf_s314");

// Wide range
invention_test_10x!(test_scalar_leaf_s8675309,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-range", 0, 1, 10, 1, 8, 8675309,
    "scalar_leaf_s8675309");

// ---------------------------------------------------------------------------
// Scalar Branch tests (depth>=1)
// ---------------------------------------------------------------------------

// Default widths, depth 1
invention_test_10x!(test_scalar_branch_s42,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-default", 1, 3, 5, 3, 5, 42,
    "scalar_branch_s42");

// Minimum width, depth 1
invention_test_10x!(test_scalar_branch_s13,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-min-1", 1, 1, 1, 1, 1, 13,
    "scalar_branch_s13");

// Narrow: exactly 2 tasks
invention_test_10x!(test_scalar_branch_s2718,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-narrow", 1, 2, 2, 2, 2, 2718,
    "scalar_branch_s2718");

// Large width, depth 2
invention_test_10x!(test_scalar_branch_s77777,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-wide-d2", 2, 10, 10, 10, 10, 77777,
    "scalar_branch_s77777");

// Asymmetric: wide branch, narrow leaf
invention_test_10x!(test_scalar_branch_s555,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-asym", 1, 8, 10, 1, 2, 555,
    "scalar_branch_s555");

// Deep depth 3, narrow
invention_test_10x!(test_scalar_branch_s161803,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-deep", 3, 2, 3, 2, 3, 161803,
    "scalar_branch_s161803");

// ---------------------------------------------------------------------------
// Vector Leaf tests (depth=0)
// ---------------------------------------------------------------------------

// Default widths
invention_test_10x!(test_vector_leaf_s42,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-default", 0, 3, 5, 3, 5, 42,
    "vector_leaf_s42");

// Minimum width
invention_test_10x!(test_vector_leaf_s23,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-min-1", 0, 1, 1, 1, 1, 23,
    "vector_leaf_s23");

// Narrow: exactly 2
invention_test_10x!(test_vector_leaf_s404,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-narrow", 0, 2, 2, 2, 2, 404,
    "vector_leaf_s404");

// Large width
invention_test_10x!(test_vector_leaf_s31415,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-wide-10", 0, 10, 10, 10, 10, 31415,
    "vector_leaf_s31415");

// Asymmetric
invention_test_10x!(test_vector_leaf_s65536,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-asym", 0, 2, 3, 6, 10, 65536,
    "vector_leaf_s65536");

// Wide range
invention_test_10x!(test_vector_leaf_s271828,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-range", 0, 1, 10, 1, 10, 271828,
    "vector_leaf_s271828");

// ---------------------------------------------------------------------------
// Vector Branch tests (depth>=1)
// ---------------------------------------------------------------------------

// Default widths, depth 1
invention_test_10x!(test_vector_branch_s42,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-default", 1, 3, 5, 3, 5, 42,
    "vector_branch_s42");

// Minimum width
invention_test_10x!(test_vector_branch_s71,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-min-1", 1, 1, 1, 1, 1, 71,
    "vector_branch_s71");

// Narrow: exactly 2
invention_test_10x!(test_vector_branch_s12345,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-narrow", 1, 2, 2, 2, 2, 12345,
    "vector_branch_s12345");

// Large width, depth 2
invention_test_10x!(test_vector_branch_s90210,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-wide-d2", 2, 10, 10, 10, 10, 90210,
    "vector_branch_s90210");

// Asymmetric: narrow branch, wide leaf
invention_test_10x!(test_vector_branch_s1984,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-asym", 1, 1, 2, 8, 10, 1984,
    "vector_branch_s1984");

// Deep depth 3
invention_test_10x!(test_vector_branch_s2025,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-deep", 3, 2, 4, 2, 4, 2025,
    "vector_branch_s2025");

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Scalar Leaf
// ---------------------------------------------------------------------------

// anyOf(string | image | nested object) + array + integer
invention_test_10x_schema!(test_scalar_leaf_schema_anyof,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-anyof", 0, 3, 5, 3, 5, 50001,
    "scalar_leaf_schema_anyof",
    scalar_schema_anyof_chaos());

// 4-level deep nested objects with media leaves
invention_test_10x_schema!(test_scalar_leaf_schema_deep,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-deep-media", 0, 2, 4, 2, 4, 60002,
    "scalar_leaf_schema_deep",
    scalar_schema_deep_media());

// array of anyOf(string, object with nested arrays + image)
invention_test_10x_schema!(test_scalar_leaf_schema_arraymad,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-arr-mad", 0, 1, 3, 1, 3, 70003,
    "scalar_leaf_schema_arraymad",
    scalar_schema_array_madness());

// every type + anyOf array + nested objects
invention_test_10x_schema!(test_scalar_leaf_schema_kitchen,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "sl-kitchen", 0, 3, 5, 3, 5, 80004,
    "scalar_leaf_schema_kitchen",
    scalar_schema_kitchen_sink());

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Scalar Branch
// ---------------------------------------------------------------------------

invention_test_10x_schema!(test_scalar_branch_schema_anyof,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-anyof", 1, 2, 3, 2, 3, 50005,
    "scalar_branch_schema_anyof",
    scalar_schema_anyof_chaos());

invention_test_10x_schema!(test_scalar_branch_schema_deep,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-deep-media", 1, 1, 2, 1, 2, 60006,
    "scalar_branch_schema_deep",
    scalar_schema_deep_media());

invention_test_10x_schema!(test_scalar_branch_schema_arraymad,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-arr-mad", 1, 2, 4, 2, 4, 70007,
    "scalar_branch_schema_arraymad",
    scalar_schema_array_madness());

invention_test_10x_schema!(test_scalar_branch_schema_kitchen,
    AlphaScalarBranch, AlphaScalarBranchState,
    "sb-kitchen", 2, 2, 3, 2, 3, 80008,
    "scalar_branch_schema_kitchen",
    scalar_schema_kitchen_sink());

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Vector Leaf
// ---------------------------------------------------------------------------

// anyOf items (image | object), context with arrays + image
invention_test_10x_schema!(test_vector_leaf_schema_multimedia,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-multimedia", 0, 1, 3, 1, 3, 50009,
    "vector_leaf_schema_multimedia",
    vector_schema_multimedia_ranking());

// items with nested anyOf arrays, context with nested objects
invention_test_10x_schema!(test_vector_leaf_schema_chaos,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-chaos", 0, 1, 2, 1, 2, 60010,
    "vector_leaf_schema_chaos",
    vector_schema_nested_chaos());

// simple string items, deeply nested context with arrays of anyOf
invention_test_10x_schema!(test_vector_leaf_schema_richctx,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-richctx", 0, 1, 2, 1, 2, 70011,
    "vector_leaf_schema_richctx",
    vector_schema_rich_context());

// no context, items with nested file/image/video anyOf + annotation arrays
invention_test_10x_schema!(test_vector_leaf_schema_noctx,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "vl-noctx", 0, 1, 2, 1, 2, 80012,
    "vector_leaf_schema_noctx",
    vector_schema_no_context_deep_items());

// ---------------------------------------------------------------------------
// Pre-provided schema tests — Vector Branch
// ---------------------------------------------------------------------------

invention_test_10x_schema!(test_vector_branch_schema_multimedia,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-multimedia", 1, 1, 2, 1, 2, 50013,
    "vector_branch_schema_multimedia",
    vector_schema_multimedia_ranking());

invention_test_10x_schema!(test_vector_branch_schema_chaos,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-chaos", 1, 1, 2, 1, 2, 60014,
    "vector_branch_schema_chaos",
    vector_schema_nested_chaos());

invention_test_10x_schema!(test_vector_branch_schema_richctx,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-richctx", 1, 1, 2, 1, 2, 70015,
    "vector_branch_schema_richctx",
    vector_schema_rich_context());

invention_test_10x_schema!(test_vector_branch_schema_noctx,
    AlphaVectorBranch, AlphaVectorBranchState,
    "vb-noctx", 1, 1, 2, 1, 2, 80016,
    "vector_branch_schema_noctx",
    vector_schema_no_context_deep_items());

// ---------------------------------------------------------------------------
// Validation error tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_zero_leaf_width_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("bad-zero", 0, 3, 5, 0, 0),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "zero leaf width should be rejected");
}

#[tokio::test]
async fn test_zero_branch_width_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let request = make_request(
        ParamsState::AlphaScalarBranch(AlphaScalarBranchState {
            params: params("bad-zero-branch", 1, 0, 0, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        2,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "zero branch width should be rejected");
}

#[tokio::test]
async fn test_min_greater_than_max_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let request = make_request(
        ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
            params: params("bad-inverted", 0, 5, 3, 5, 3),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        3,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "min > max should be rejected");
}

// ---------------------------------------------------------------------------
// Completed state test — no completions should be generated
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_completed_state_generates_no_completions() {
    // Load a fully completed snapshot and extract its state as ParamsState.
    let snapshot: serde_json::Value = serde_json::from_str(
        include_str!("../../../assets/functions/inventions/client_tests/scalar_leaf_s42_0.json"),
    ).unwrap();
    let state: ParamsState = serde_json::from_value(snapshot["state"].clone()).unwrap();

    let client = make_client();
    let request = make_request(state, 42);
    let result = run_invention(&client, request).await;

    assert!(
        result.completions.is_empty(),
        "completed state should generate no completions, got {}",
        result.completions.len(),
    );
}

// ---------------------------------------------------------------------------
// InvalidName tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_name_over_100_bytes_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let long_name = "a".repeat(101);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&long_name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "name over 100 bytes should be rejected");
}

#[tokio::test]
async fn test_name_without_path_over_77_bytes_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    // 78 bytes, no `-` path segment
    let name = "a".repeat(78);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "name without path segment over 77 bytes should be rejected");
}

#[tokio::test]
async fn test_name_without_path_at_77_bytes_accepted() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    let name = "a".repeat(77);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_ok(), "name at exactly 77 bytes without path should be accepted");
}

#[tokio::test]
async fn test_name_with_valid_path_over_77_bytes_accepted() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    // 80 bytes before `-`, then a valid b62 path segment: "1" encodes path [0]
    let name = format!("{}-1", "a".repeat(80));
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_ok(), "name with valid path segment over 77 bytes should be accepted");
}

#[tokio::test]
async fn test_name_78_bytes_ending_in_dash_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    // 77 'a's + '-' = 78 bytes, empty segment after dash is not a valid b62 path
    let name = format!("{}-", "a".repeat(77));
    assert_eq!(name.len(), 78);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "78-byte name ending in '-' should be rejected");
}

#[tokio::test]
async fn test_name_100_bytes_ending_in_dash_rejected() {
    let client = make_client();
    let ctx = ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new());
    // 99 'a's + '-' = 100 bytes, empty segment after dash is not a valid b62 path
    let name = format!("{}-", "a".repeat(99));
    assert_eq!(name.len(), 100);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let result = client.create_streaming(ctx, request).await;
    assert!(result.is_err(), "100-byte name ending in '-' should be rejected");
}

