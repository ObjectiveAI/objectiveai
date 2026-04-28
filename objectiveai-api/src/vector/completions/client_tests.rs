use std::sync::Arc;
use std::time::Duration;

use rust_decimal::Decimal;

use objectiveai::agent::completions::message::{
    File as MessageFile, ImageUrl, Message, RichContent, RichContentPart, UserMessage, VideoUrl,
};
use objectiveai::agent::mock::{AgentBase as MockAgentBase, OutputMode as MockOutputMode, Upstream as MockUpstream};
use objectiveai::vector::completions::response::unary::VectorCompletion;

use crate::agent::completions::UnimplementedUpstreamClient;
use crate::ctx;

// ---------------------------------------------------------------------------
// Stubs — never actually called since we always provide inline mock agents.
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

struct StubCompletionVotesFetcher;

#[async_trait::async_trait]
impl super::completion_votes_fetcher::Fetcher<ctx::DefaultContextExt>
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
impl super::cache_vote_fetcher::Fetcher<ctx::DefaultContextExt>
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
impl super::usage_handler::UsageHandler<ctx::DefaultContextExt>
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

// ---------------------------------------------------------------------------
// Shared helpers for constructing clients
// ---------------------------------------------------------------------------

fn make_retrieve_router() -> Arc<crate::retrieval::retrieve::Router<StubRetrieveClient, StubRetrieveClient, crate::retrieval::retrieve::mock::MockClient, ctx::DefaultContextExt>> {
    Arc::new(crate::retrieval::retrieve::Router::new(
        Arc::new(StubRetrieveClient),
        Arc::new(StubRetrieveClient),
        Arc::new(crate::retrieval::retrieve::mock::MockClient),
    ))
}

fn make_agent_client(
    retrieve_router: &Arc<crate::retrieval::retrieve::Router<StubRetrieveClient, StubRetrieveClient, crate::retrieval::retrieve::mock::MockClient, ctx::DefaultContextExt>>,
) -> Arc<crate::agent::completions::Client<ctx::DefaultContextExt, UnimplementedUpstreamClient, UnimplementedUpstreamClient, crate::agent::completions::mock::Client, StubRetrieveClient, StubRetrieveClient, crate::retrieval::retrieve::mock::MockClient, StubAgentUsageHandler>> {
    Arc::new(crate::agent::completions::Client::new(
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
    ))
}

type TestVectorClient = super::Client<
    ctx::DefaultContextExt,
    UnimplementedUpstreamClient,
    UnimplementedUpstreamClient,
    crate::agent::completions::mock::Client,
    StubRetrieveClient,
    StubRetrieveClient,
    crate::retrieval::retrieve::mock::MockClient,
    StubAgentUsageHandler,
    StubCompletionVotesFetcher,
    StubCacheVoteFetcher,
    StubVectorUsageHandler,
>;

fn make_vector_client() -> Arc<TestVectorClient> {
    let retrieve_router = make_retrieve_router();
    let agent_client = make_agent_client(&retrieve_router);
    Arc::new(super::Client {
        agent_client,
        retrieve_router,
        completion_votes_fetcher: Arc::new(StubCompletionVotesFetcher),
        cache_vote_fetcher: Arc::new(StubCacheVoteFetcher),
        usage_handler: Arc::new(StubVectorUsageHandler),
    })
}

/// Helper to construct a mock agent for swarms.
fn mock_agent(
    output_mode: MockOutputMode,
    count: u64,
    top_logprobs: Option<u64>,
    error: Option<bool>,
    fallbacks: Option<Vec<objectiveai::agent::InlineAgentBase>>,
) -> objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
    objectiveai::agent::InlineAgentBaseWithFallbacksOrRemoteWithCount {
        count,
        inner: objectiveai::agent::InlineAgentBaseWithFallbacksOrRemote::AgentBase(
            objectiveai::agent::InlineAgentBaseWithFallbacks {
                inner: objectiveai::agent::InlineAgentBase::Mock(MockAgentBase {
                    upstream: MockUpstream::Mock,
                    output_mode,
                    top_logprobs,
                    error,
                    error_probability: None,
                    mode: None,
                    mcp_servers: None,
                }),
                fallbacks,
            },
        ),
    }
}

// ---------------------------------------------------------------------------
// Snapshot helpers
// ---------------------------------------------------------------------------

fn check_created(expected: &std::cell::Cell<Option<u64>>, i: usize, created: u64) {
    match expected.get() {
        None => expected.set(Some(created)),
        Some(exp) => assert_eq!(created, exp, "chunk {i} has created {created}, expected {exp}"),
    }
}

async fn run_and_check(
    stream: impl futures::Stream<
        Item = objectiveai::vector::completions::response::streaming::VectorCompletionChunk,
    > + Unpin,
) -> VectorCompletion {
    let expected_created = std::cell::Cell::new(None);
    let agg = crate::stream_harness::consume_stream(
        stream,
        |agg, c| agg.push(c),
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.completions.len() <= 1, "chunk {i} has {} completions, expected at most 1", chunk.completions.len());
            assert!(chunk.usage.is_none(), "chunk {i} (non-final) has usage, expected None");
        },
        |i, chunk| {
            check_created(&expected_created, i, chunk.created);
            assert!(chunk.usage.is_some(), "final chunk {i} has no usage, expected Some");
        },
    ).await;
    VectorCompletion::from(agg)
}

fn normalize(mut vc: VectorCompletion) -> VectorCompletion {
    vc.normalize_for_tests();
    vc
}

fn assert_snapshot(json: &str, path: &str, expected: &str) {
    crate::stream_harness::assert_snapshot(
        json, path, expected,
        "UPDATE_VECTOR_COMPLETIONS_CLIENT_TESTS_SNAPSHOTS",
    );
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Single mock agent, 2 responses, instruction mode, seed 42.
#[tokio::test]
async fn test_single_agent_2_responses_instruction_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which is better?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Response A".to_string()),
            RichContent::Text("Response B".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/single_agent_2_responses_instruction_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/single_agent_2_responses_instruction_seed_42.json"),
    );
}

/// Single mock agent, 3 responses, instruction mode, seed 42.
#[tokio::test]
async fn test_single_agent_3_responses_instruction_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which is best?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Alpha".to_string()),
            RichContent::Text("Beta".to_string()),
            RichContent::Text("Gamma".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/single_agent_3_responses_instruction_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/single_agent_3_responses_instruction_seed_42.json"),
    );
}

/// Two mock agents with equal weights, seed 42.
#[tokio::test]
async fn test_two_agents_equal_weights_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick one".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Option 1".to_string()),
            RichContent::Text("Option 2".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/two_agents_equal_weights_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/two_agents_equal_weights_seed_42.json"),
    );
}

/// Two different mock agent definitions with unequal weights (0.8 / 0.2), seed 42.
#[tokio::test]
async fn test_two_agents_unequal_weights_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick one".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(8, 1),
                    Decimal::new(2, 1),
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Option 1".to_string()),
            RichContent::Text("Option 2".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/two_agents_unequal_weights_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/two_agents_unequal_weights_seed_42.json"),
    );
}

/// Three agents (via count=3), 4 responses, seed 99.
#[tokio::test]
async fn test_three_agents_4_responses_seed_99() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rank these".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 3, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(99),
        stream: None,
        responses: vec![
            RichContent::Text("Red".to_string()),
            RichContent::Text("Green".to_string()),
            RichContent::Text("Blue".to_string()),
            RichContent::Text("Yellow".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 6);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/three_agents_4_responses_seed_99.json"),
        include_str!("../../../assets/vector/completions/client_tests/three_agents_4_responses_seed_99.json"),
    );
}

/// Invert vote with single agent, seed 42.
#[tokio::test]
async fn test_invert_vote_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which is worse?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Entries(vec![
                    objectiveai::WeightsEntry {
                        weight: Decimal::ONE,
                        invert: Some(true),
                    },
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Bad option".to_string()),
            RichContent::Text("Worse option".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/invert_vote_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/invert_vote_seed_42.json"),
    );
}

/// Same seed produces same result (deterministic).
#[tokio::test]
async fn test_deterministic_same_seed() {
    let make_request = || {
        Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
            retry: None,
            from_cache: None,
            messages: vec![Message::User(UserMessage {
                content: RichContent::Text("Pick one".to_string()),
                name: None,
            })],
            provider: None,
            swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
                objectiveai::swarm::InlineSwarmBase {
                    agents: vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
                    weights: Some(objectiveai::Weights::Weights(vec![
                        Decimal::ONE,
                    ])),
                },
            ),
            seed: Some(42),
            stream: None,
            responses: vec![
                RichContent::Text("A".to_string()),
                RichContent::Text("B".to_string()),
                RichContent::Text("C".to_string()),
            ],
            continuation: None,
            })
    };

    let run = |client: Arc<TestVectorClient>, request| async move {
        let stream = client
            .create_streaming(
                ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
                request,
            )
            .await
            .expect("should succeed");
        normalize(run_and_check(Box::pin(stream)).await)
    };

    let result1 = run(make_vector_client(), make_request()).await;
    let result2 = run(make_vector_client(), make_request()).await;

    let json1 = serde_json::to_string_pretty(&result1).unwrap();
    let json2 = serde_json::to_string_pretty(&result2).unwrap();
    assert_eq!(json1, json2, "same seed should produce identical results");
}

/// Different seeds produce different results.
#[tokio::test]
async fn test_different_seeds_differ() {
    let client = make_vector_client();
    let make_request = |seed: i64| {
        Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
            retry: None,
            from_cache: None,
            messages: vec![Message::User(UserMessage {
                content: RichContent::Text("Pick one".to_string()),
                name: None,
            })],
            provider: None,
            swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
                objectiveai::swarm::InlineSwarmBase {
                    agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                    weights: Some(objectiveai::Weights::Weights(vec![
                        Decimal::ONE,
                    ])),
                },
            ),
            seed: Some(seed),
            stream: None,
            responses: vec![
                RichContent::Text("A".to_string()),
                RichContent::Text("B".to_string()),
            ],
            continuation: None,
            })
    };

    let run = |client: Arc<TestVectorClient>, request| async move {
        let stream = client
            .create_streaming(
                ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
                request,
            )
            .await
            .expect("should succeed");
        normalize(run_and_check(Box::pin(stream)).await)
    };

    let result1 = run(client.clone(), make_request(42)).await;
    let result2 = run(client.clone(), make_request(99)).await;

    let json1 = serde_json::to_string_pretty(&result1).unwrap();
    let json2 = serde_json::to_string_pretty(&result2).unwrap();
    assert_ne!(json1, json2, "different seeds should produce different results");
}

/// Many responses (25) to test deep prefix tree, seed 42.
#[tokio::test]
async fn test_many_responses_deep_prefix_tree_seed_42() {
    let client = make_vector_client();
    let responses: Vec<RichContent> = (0..25)
        .map(|i| RichContent::Text(format!("Response {}", i)))
        .collect();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick the best".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses,
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/many_responses_deep_prefix_tree_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/many_responses_deep_prefix_tree_seed_42.json"),
    );
}

/// Single agent with json_schema output mode, seed 77.
#[tokio::test]
async fn test_json_schema_single_agent_seed_77() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rate the following essays on clarity".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(77),
        stream: None,
        responses: vec![
            RichContent::Text("Essay about climate change".to_string()),
            RichContent::Text("Essay about artificial intelligence".to_string()),
            RichContent::Text("Essay about space exploration".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/json_schema_single_agent_seed_77.json"),
        include_str!("../../../assets/vector/completions/client_tests/json_schema_single_agent_seed_77.json"),
    );
}

/// Single agent with tool_call output mode, seed 55.
#[tokio::test]
async fn test_tool_call_single_agent_seed_55() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which logo design is most memorable?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::ToolCall, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(55),
        stream: None,
        responses: vec![
            RichContent::Text("Minimalist wordmark".to_string()),
            RichContent::Text("Abstract geometric icon".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert!(result.completions.len() >= 1, "should have at least one completion");
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/tool_call_single_agent_seed_55.json"),
        include_str!("../../../assets/vector/completions/client_tests/tool_call_single_agent_seed_55.json"),
    );
}

/// Single error agent — completion should contain an error, no votes.
#[tokio::test]
async fn test_error_agent_skipped_seed_42() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Evaluate these proposals".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, Some(true), None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Proposal A".to_string()),
            RichContent::Text("Proposal B".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/error_agent_skipped_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/error_agent_skipped_seed_42.json"),
    );
}

/// Mixed output modes: instruction + json_schema + tool_call agents, seed 88.
#[tokio::test]
async fn test_mixed_output_modes_seed_88() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Compare these vacation destinations".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                    mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(4, 1),
                    Decimal::new(3, 1),
                    Decimal::new(3, 1),
                ])),
            },
        ),
        seed: Some(88),
        stream: None,
        responses: vec![
            RichContent::Text("Kyoto, Japan".to_string()),
            RichContent::Text("Reykjavik, Iceland".to_string()),
            RichContent::Text("Patagonia, Argentina".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/mixed_output_modes_seed_88.json"),
        include_str!("../../../assets/vector/completions/client_tests/mixed_output_modes_seed_88.json"),
    );
}

/// Image responses with instruction mode, seed 33.
#[tokio::test]
async fn test_image_responses_instruction_seed_33() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which painting has the best composition?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(33),
        stream: None,
        responses: vec![
            RichContent::Parts(vec![
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/painting-a.jpg".to_string(),
                        detail: None,
                    },
                },
                RichContentPart::Text { text: "Sunset over mountains".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/painting-b.jpg".to_string(),
                        detail: None,
                    },
                },
                RichContentPart::Text { text: "Abstract cubist portrait".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/painting-c.jpg".to_string(),
                        detail: None,
                    },
                },
                RichContentPart::Text { text: "Watercolor garden scene".to_string() },
            ]),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/image_responses_instruction_seed_33.json"),
        include_str!("../../../assets/vector/completions/client_tests/image_responses_instruction_seed_33.json"),
    );
}

/// Video and file responses with json_schema mode, seed 66.
#[tokio::test]
async fn test_video_and_file_responses_seed_66() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Parts(vec![
                RichContentPart::Text { text: "Review these submissions and pick the best one".to_string() },
                RichContentPart::VideoUrl {
                    video_url: VideoUrl {
                        url: "https://example.com/demo-reel.mp4".to_string(),
                    },
                },
            ]),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(66),
        stream: None,
        responses: vec![
            RichContent::Parts(vec![
                RichContentPart::VideoUrl {
                    video_url: VideoUrl {
                        url: "https://example.com/submission-1.mp4".to_string(),
                    },
                },
                RichContentPart::Text { text: "30-second product demo".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::File {
                    file: MessageFile {
                        file_data: None,
                        file_id: None,
                        filename: Some("business-plan.pdf".to_string()),
                        file_url: Some("https://example.com/business-plan.pdf".to_string()),
                    },
                },
                RichContentPart::Text { text: "Written business plan".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::VideoUrl {
                    video_url: VideoUrl {
                        url: "https://example.com/submission-3.mp4".to_string(),
                    },
                },
                RichContentPart::File {
                    file: MessageFile {
                        file_data: None,
                        file_id: None,
                        filename: Some("appendix.pdf".to_string()),
                        file_url: Some("https://example.com/appendix.pdf".to_string()),
                    },
                },
            ]),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 1);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/video_and_file_responses_seed_66.json"),
        include_str!("../../../assets/vector/completions/client_tests/video_and_file_responses_seed_66.json"),
    );
}

/// Three distinct agent definitions (instruction, json_schema, tool_call), seed 11.
#[tokio::test]
async fn test_three_different_agents_seed_11() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Parts(vec![
                RichContentPart::Text { text: "Which dish looks the most appetizing?".to_string() },
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/menu-context.jpg".to_string(),
                        detail: None,
                    },
                },
            ]),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                    mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(5, 1),
                    Decimal::new(3, 1),
                    Decimal::new(2, 1),
                ])),
            },
        ),
        seed: Some(11),
        stream: None,
        responses: vec![
            RichContent::Text("Truffle risotto".to_string()),
            RichContent::Text("Seared tuna tataki".to_string()),
            RichContent::Text("Wagyu beef carpaccio".to_string()),
            RichContent::Text("Lobster thermidor".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert!(result.completions.len() >= 3, "should have at least one completion per agent");
    assert_eq!(result.votes.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/three_different_agents_seed_11.json"),
        include_str!("../../../assets/vector/completions/client_tests/three_different_agents_seed_11.json"),
    );
}

/// Json_schema mode with 8 responses, seed 22.
#[tokio::test]
async fn test_json_schema_many_responses_seed_22() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rank these programming languages by expressiveness".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 2, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(22),
        stream: None,
        responses: vec![
            RichContent::Text("Rust".to_string()),
            RichContent::Text("Haskell".to_string()),
            RichContent::Text("Python".to_string()),
            RichContent::Text("Lisp".to_string()),
            RichContent::Text("APL".to_string()),
            RichContent::Text("Forth".to_string()),
            RichContent::Text("Prolog".to_string()),
            RichContent::Text("Smalltalk".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 2);
    assert_eq!(result.votes.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/json_schema_many_responses_seed_22.json"),
        include_str!("../../../assets/vector/completions/client_tests/json_schema_many_responses_seed_22.json"),
    );
}

/// Two tool_call agents with image message, seed 44.
#[tokio::test]
async fn test_tool_call_two_agents_seed_44() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Parts(vec![
                RichContentPart::Text { text: "Which UI mockup should we go with?".to_string() },
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/current-design.png".to_string(),
                        detail: None,
                    },
                },
            ]),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
                    mock_agent(MockOutputMode::ToolCall, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(6, 1),
                    Decimal::new(4, 1),
                ])),
            },
        ),
        seed: Some(44),
        stream: None,
        responses: vec![
            RichContent::Parts(vec![
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/mockup-a.png".to_string(),
                        detail: None,
                    },
                },
                RichContentPart::Text { text: "Clean flat design".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: "https://example.com/mockup-b.png".to_string(),
                        detail: None,
                    },
                },
                RichContentPart::Text { text: "Skeuomorphic with gradients".to_string() },
            ]),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/tool_call_two_agents_seed_44.json"),
        include_str!("../../../assets/vector/completions/client_tests/tool_call_two_agents_seed_44.json"),
    );
}

/// One error agent + two healthy agents (json_schema, instruction), seed 99.
#[tokio::test]
async fn test_error_and_healthy_agents_seed_99() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Parts(vec![
                RichContentPart::Text { text: "Evaluate these architectural plans".to_string() },
                RichContentPart::File {
                    file: MessageFile {
                        file_data: None,
                        file_id: None,
                        filename: Some("site-survey.pdf".to_string()),
                        file_url: Some("https://example.com/site-survey.pdf".to_string()),
                    },
                },
            ]),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, Some(true), None),
                    mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(3, 1),
                    Decimal::new(4, 1),
                    Decimal::new(3, 1),
                ])),
            },
        ),
        seed: Some(99),
        stream: None,
        responses: vec![
            RichContent::Parts(vec![
                RichContentPart::File {
                    file: MessageFile {
                        file_data: None,
                        file_id: None,
                        filename: Some("plan-modern.pdf".to_string()),
                        file_url: Some("https://example.com/plan-modern.pdf".to_string()),
                    },
                },
                RichContentPart::Text { text: "Modern glass facade".to_string() },
            ]),
            RichContent::Parts(vec![
                RichContentPart::File {
                    file: MessageFile {
                        file_data: None,
                        file_id: None,
                        filename: Some("plan-traditional.pdf".to_string()),
                        file_url: Some("https://example.com/plan-traditional.pdf".to_string()),
                    },
                },
                RichContentPart::Text { text: "Traditional brick and stone".to_string() },
            ]),
            RichContent::Text("Brutalist concrete monolith".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.completions.len(), 4);
    assert_eq!(result.votes.len(), 1);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/error_and_healthy_agents_seed_99.json"),
        include_str!("../../../assets/vector/completions/client_tests/error_and_healthy_agents_seed_99.json"),
    );
}

/// Only the final chunk should carry usage; all earlier chunks should have usage: None.
#[tokio::test]
async fn test_only_final_chunk_has_usage() {
    let client = make_vector_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick one".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 2, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("A".to_string()),
            RichContent::Text("B".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    run_and_check(Box::pin(stream)).await;
}

// ---------------------------------------------------------------------------
// Error tests
// ---------------------------------------------------------------------------

/// Helper to build a client for error tests (no snapshot needed).
fn make_error_test_client() -> Arc<TestVectorClient> {
    make_vector_client()
}

/// Zero responses → ExpectedTwoOrMoreRequestVectorResponses(0).
#[tokio::test]
async fn test_error_zero_responses() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick one".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with zero responses");
    let msg = err.to_string();
    assert!(
        msg.contains("expected two or more") && msg.contains("got 0"),
        "unexpected error: {msg}"
    );
}

/// One response → ExpectedTwoOrMoreRequestVectorResponses(1).
#[tokio::test]
async fn test_error_one_response() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rate this".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Only option".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with one response");
    let msg = err.to_string();
    assert!(
        msg.contains("expected two or more") && msg.contains("got 1"),
        "unexpected error: {msg}"
    );
}

/// All agents have count=0 → InvalidSwarm (no agents after filtering).
#[tokio::test]
async fn test_error_invalid_swarm_all_count_zero() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Compare".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 0, None, None, None),
                    mock_agent(MockOutputMode::ToolCall, 0, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(5, 1),
                    Decimal::new(5, 1),
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("A".to_string()),
            RichContent::Text("B".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with all count-0 agents");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid swarm"),
        "unexpected error: {msg}"
    );
}

/// Empty agents vec → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_empty_agents() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which is better?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![],
                weights: Some(objectiveai::Weights::Weights(vec![])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("X".to_string()),
            RichContent::Text("Y".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with empty agents");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid swarm"),
        "unexpected error: {msg}"
    );
}

/// Profile length doesn't match agents length → InvalidSwarm (caught by try_from_with_profile).
#[tokio::test]
async fn test_error_invalid_swarm_profile_length_mismatch() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Choose".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("A".to_string()),
            RichContent::Text("B".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with profile/agents length mismatch");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid swarm") && msg.contains("does not match"),
        "unexpected error: {msg}"
    );
}

/// Duplicate agents with conflicting invert flags → InvalidSwarm.
#[tokio::test]
async fn test_error_invalid_swarm_conflicting_invert() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rank these".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                    // Same agent definition — will be merged, but conflicting invert flags
                    mock_agent(MockOutputMode::Instruction, 1, None, None, None),
                ],
                weights: Some(objectiveai::Weights::Entries(vec![
                    objectiveai::WeightsEntry {
                        weight: Decimal::new(5, 1),
                        invert: Some(false),
                    },
                    objectiveai::WeightsEntry {
                        weight: Decimal::new(5, 1),
                        invert: Some(true),
                    },
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("A".to_string()),
            RichContent::Text("B".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with conflicting invert flags");
    let msg = err.to_string();
    assert!(
        msg.contains("invalid swarm") && msg.contains("conflicting invert"),
        "unexpected error: {msg}"
    );
}

/// All weights are zero → error during swarm conversion.
#[tokio::test]
async fn test_error_invalid_profile_all_zero_weights() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Score these".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::ToolCall, 1, None, None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ZERO,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("A".to_string()),
            RichContent::Text("B".to_string()),
        ],
        continuation: None,
    });

    let err = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .err()
        .expect("should fail with all-zero weights");
    let msg = err.to_string();
    assert!(
        msg.contains("at least one positive"),
        "unexpected error: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Logprobs tests
// ---------------------------------------------------------------------------

/// JsonSchema output mode with logprobs, 2 agents, 3 responses.
#[tokio::test]
async fn test_logprobs_json_schema_2_agents_seed_42() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rate these options".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(5), None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(5), None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(6, 1),
                    Decimal::new(4, 1),
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("Option A".to_string()),
            RichContent::Text("Option B".to_string()),
            RichContent::Text("Option C".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_json_schema_2_agents_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_json_schema_2_agents_seed_42.json"),
    );
}

/// JsonSchema, 3 agents with unequal weights, 4 responses, high top_logprobs.
#[tokio::test]
async fn test_logprobs_json_schema_3_agents_unequal_seed_77() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Rank these candidates".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(10), None, None),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(5, 1),
                    Decimal::new(3, 1),
                    Decimal::new(2, 1),
                ])),
            },
        ),
        seed: Some(77),
        stream: None,
        responses: vec![
            RichContent::Text("Candidate Alpha".to_string()),
            RichContent::Text("Candidate Beta".to_string()),
            RichContent::Text("Candidate Gamma".to_string()),
            RichContent::Text("Candidate Delta".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 4);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_json_schema_3_agents_unequal_seed_77.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_json_schema_3_agents_unequal_seed_77.json"),
    );
}

/// ToolCall output mode with logprobs, single agent.
#[tokio::test]
async fn test_logprobs_tool_call_single_agent_seed_55() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Pick the best tool".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::ToolCall, 1, Some(3), None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(55),
        stream: None,
        responses: vec![
            RichContent::Text("Hammer".to_string()),
            RichContent::Text("Screwdriver".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_tool_call_single_agent_seed_55.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_tool_call_single_agent_seed_55.json"),
    );
}

/// Error primary agent with healthy logprobs-enabled fallback.
#[tokio::test]
async fn test_logprobs_error_with_fallback_seed_99() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Score these".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 1, Some(8), Some(true), Some(vec![
                    objectiveai::agent::InlineAgentBase::Mock(MockAgentBase {
                        upstream: MockUpstream::Mock,
                        output_mode: MockOutputMode::JsonSchema,
                        top_logprobs: Some(8),
                        error: None,
                        error_probability: None,
                        mode: None,
                        mcp_servers: None,
                    }),
                ]))],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(99),
        stream: None,
        responses: vec![
            RichContent::Text("Plan A".to_string()),
            RichContent::Text("Plan B".to_string()),
            RichContent::Text("Plan C".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("fallback should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_error_with_fallback_seed_99.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_error_with_fallback_seed_99.json"),
    );
}

/// Both primary and fallback error — should produce error completion, no votes.
#[tokio::test]
async fn test_logprobs_all_errors_seed_42() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Evaluate".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::JsonSchema, 1, Some(5), Some(true), Some(vec![
                    objectiveai::agent::InlineAgentBase::Mock(MockAgentBase {
                        upstream: MockUpstream::Mock,
                        output_mode: MockOutputMode::ToolCall,
                        top_logprobs: Some(3),
                        error: Some(true),
                        error_probability: None,
                        mode: None,
                        mcp_servers: None,
                    }),
                ]))],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(42),
        stream: None,
        responses: vec![
            RichContent::Text("X".to_string()),
            RichContent::Text("Y".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("create_streaming should succeed even with all errors");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    // All agents errored — no votes produced.
    assert_eq!(result.votes.len(), 0);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_all_errors_seed_42.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_all_errors_seed_42.json"),
    );
}

/// Instruction output mode with logprobs (rare combination).
#[tokio::test]
async fn test_logprobs_instruction_seed_33() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Which do you prefer?".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![mock_agent(MockOutputMode::Instruction, 1, Some(2), None, None)],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::ONE,
                ])),
            },
        ),
        seed: Some(33),
        stream: None,
        responses: vec![
            RichContent::Text("Cats".to_string()),
            RichContent::Text("Dogs".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("instruction with logprobs should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 2);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_instruction_seed_33.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_instruction_seed_33.json"),
    );
}

/// Mixed: response_format + tool_call + instruction agents, one with error+fallback.
#[tokio::test]
async fn test_logprobs_mixed_modes_with_fallback_seed_88() {
    let client = make_error_test_client();
    let request = Arc::new(objectiveai::vector::completions::request::VectorCompletionCreateParams {
        retry: None,
        from_cache: None,
        messages: vec![Message::User(UserMessage {
            content: RichContent::Text("Compare these designs".to_string()),
            name: None,
        })],
        provider: None,
        swarm: objectiveai::swarm::InlineSwarmBaseOrRemoteCommitOptional::SwarmBase(
            objectiveai::swarm::InlineSwarmBase {
                agents: vec![
                    // JsonSchema agent with logprobs.
                    mock_agent(MockOutputMode::JsonSchema, 1, Some(6), None, None),
                    // ToolCall agent with logprobs.
                    mock_agent(MockOutputMode::ToolCall, 1, Some(4), None, None),
                    // Error primary, healthy instruction fallback with logprobs.
                    mock_agent(MockOutputMode::Instruction, 1, Some(3), Some(true), Some(vec![
                        objectiveai::agent::InlineAgentBase::Mock(MockAgentBase {
                            upstream: MockUpstream::Mock,
                            output_mode: MockOutputMode::Instruction,
                            top_logprobs: Some(3),
                            error: None,
                            error_probability: None,
                            mode: None,
                            mcp_servers: None,
                        }),
                    ])),
                ],
                weights: Some(objectiveai::Weights::Weights(vec![
                    Decimal::new(4, 1),
                    Decimal::new(4, 1),
                    Decimal::new(2, 1),
                ])),
            },
        ),
        seed: Some(88),
        stream: None,
        responses: vec![
            RichContent::Text("Design Minimal".to_string()),
            RichContent::Text("Design Ornate".to_string()),
            RichContent::Text("Design Hybrid".to_string()),
        ],
        continuation: None,
    });

    let stream = client
        .create_streaming(
            ctx::Context::new(Arc::new(ctx::DefaultContextExt), Arc::new(ctx::persistent_cache::default::DefaultPersistentCacheClient), Decimal::ONE, false, &axum::http::HeaderMap::new()),
            request,
        )
        .await
        .expect("mixed modes with fallback should succeed");
    let result = normalize(run_and_check(Box::pin(stream)).await);
    assert_eq!(result.scores.len(), 3);
    // 3 agents, but the error primary should have been replaced by its fallback.
    assert!(result.completions.len() >= 3);

    let json = serde_json::to_string_pretty(&result).unwrap();
    assert_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/vector/completions/client_tests/logprobs_mixed_modes_with_fallback_seed_88.json"),
        include_str!("../../../assets/vector/completions/client_tests/logprobs_mixed_modes_with_fallback_seed_88.json"),
    );
}

#[test]
fn invert_and_l1_normalize_example() {
    use rust_decimal::dec;
    let v = vec![dec!(0.75), dec!(0.25), dec!(0.0)];
    let out = super::invert_and_l1_normalize(v);
    assert_eq!(out, vec![dec!(0.125), dec!(0.375), dec!(0.5)]);
}

#[test]
fn invert_and_l1_normalize_uniform_when_all_ones() {
    use rust_decimal::dec;
    let v = vec![dec!(1.0), dec!(1.0), dec!(1.0), dec!(1.0)];
    // invert -> all zeros -> uniform
    let out = super::invert_and_l1_normalize(v);
    assert_eq!(out, vec![dec!(0.25), dec!(0.25), dec!(0.25), dec!(0.25)]);
}
