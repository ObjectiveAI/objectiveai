//! ObjectiveAI API server.
//!
//! REST API server for chat completions, vector completions, Functions,
//! Profiles, Swarms, and authentication.

use axum::{
    Json,
    response::{IntoResponse, Sse, sse::Event},
};
use envconfig::Envconfig;
use objectiveai::error::ResponseError;
use objectiveai_api::{
    agent, auth, ctx,
    error::ResponseErrorExt,
    filesystem,
    functions::{self, profiles::computations::Client},
    github, mcp, objectiveai_http,
    retrieval,
    util::StreamOnce,
    vector,
};
use std::{convert::Infallible, sync::Arc};
use tokio_stream::StreamExt;

type ListRouter = retrieval::list::Router<
    retrieval::list::objectiveai::ObjectiveAiClient,
    retrieval::list::filesystem::FilesystemClient,
    retrieval::list::mock::MockClient,
    ctx::DefaultContextExt,
>;

type RetrieveRouter = retrieval::retrieve::Router<
    retrieval::retrieve::github::GithubClient,
    retrieval::retrieve::filesystem::FilesystemClient,
    retrieval::retrieve::mock::MockClient,
    ctx::DefaultContextExt,
>;

type UsageRouter = retrieval::usage::Router<
    retrieval::usage::objectiveai::ObjectiveAiClient,
    ctx::DefaultContextExt,
>;

#[derive(Envconfig)]
struct Config {
    #[envconfig(
        from = "OBJECTIVEAI_API_BASE",
        default = "https://api.objective-ai.io"
    )]
    objectiveai_api_base: String,
    #[envconfig(from = "OBJECTIVEAI_API_KEY")]
    objectiveai_api_key: Option<String>,
    #[envconfig(
        from = "OPENROUTER_API_BASE",
        default = "https://openrouter.ai/api/v1"
    )]
    openrouter_api_base: String,
    #[envconfig(from = "OPENROUTER_API_KEY")]
    openrouter_api_key: Option<String>,
    #[envconfig(from = "CLAUDE_AGENT_SDK", default = "0")]
    claude_agent_sdk: String,
    #[envconfig(from = "USER_AGENT")]
    user_agent: Option<String>,
    #[envconfig(from = "HTTP_REFERER")]
    http_referer: Option<String>,
    #[envconfig(from = "X_TITLE")]
    x_title: Option<String>,
    #[envconfig(
        from = "AGENT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    agent_completions_backoff_current_interval: u64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    agent_completions_backoff_initial_interval: u64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR",
        default = "0.5"
    )]
    agent_completions_backoff_randomization_factor: f64,
    #[envconfig(from = "AGENT_COMPLETIONS_BACKOFF_MULTIPLIER", default = "1.5")]
    agent_completions_backoff_multiplier: f64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_BACKOFF_MAX_INTERVAL",
        default = "1000" // 1 second
    )]
    agent_completions_backoff_max_interval: u64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME",
        default = "40000" // 40 seconds
    )]
    agent_completions_backoff_max_elapsed_time: u64,
    #[envconfig(
        from = "MCP_BACKOFF_CURRENT_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    mcp_backoff_current_interval: u64,
    #[envconfig(
        from = "MCP_BACKOFF_INITIAL_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    mcp_backoff_initial_interval: u64,
    #[envconfig(
        from = "MCP_BACKOFF_RANDOMIZATION_FACTOR",
        default = "0.5"
    )]
    mcp_backoff_randomization_factor: f64,
    #[envconfig(from = "MCP_BACKOFF_MULTIPLIER", default = "1.5")]
    mcp_backoff_multiplier: f64,
    #[envconfig(
        from = "MCP_BACKOFF_MAX_INTERVAL",
        default = "1000" // 1 second
    )]
    mcp_backoff_max_interval: u64,
    #[envconfig(
        from = "MCP_BACKOFF_MAX_ELAPSED_TIME",
        default = "40000" // 40 seconds
    )]
    mcp_backoff_max_elapsed_time: u64,
    #[envconfig(
        from = "GITHUB_BACKOFF_CURRENT_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    github_backoff_current_interval: u64,
    #[envconfig(
        from = "GITHUB_BACKOFF_INITIAL_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    github_backoff_initial_interval: u64,
    #[envconfig(
        from = "GITHUB_BACKOFF_RANDOMIZATION_FACTOR",
        default = "0.5"
    )]
    github_backoff_randomization_factor: f64,
    #[envconfig(from = "GITHUB_BACKOFF_MULTIPLIER", default = "1.5")]
    github_backoff_multiplier: f64,
    #[envconfig(
        from = "GITHUB_BACKOFF_MAX_INTERVAL",
        default = "1000" // 1 second
    )]
    github_backoff_max_interval: u64,
    #[envconfig(
        from = "GITHUB_BACKOFF_MAX_ELAPSED_TIME",
        default = "40000" // 40 seconds
    )]
    github_backoff_max_elapsed_time: u64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_FIRST_CHUNK_TIMEOUT",
        default = "60000" // 60 seconds
    )]
    agent_completions_first_chunk_timeout: u64,
    #[envconfig(
        from = "AGENT_COMPLETIONS_OTHER_CHUNK_TIMEOUT",
        default = "30000" // 30 seconds
    )]
    agent_completions_other_chunk_timeout: u64,
    #[envconfig(
        from = "MCP_CONNECT_TIMEOUT",
        default = "30000" // 30 seconds
    )]
    mcp_connect_timeout: u64,
    #[envconfig(
        from = "MCP_CALL_TIMEOUT",
        default = "30000" // 30 seconds
    )]
    mcp_call_timeout: u64,
    #[envconfig(from = "FETCH_GITHUB_TOKEN")]
    fetch_github_token: Option<String>,
    #[envconfig(from = "PUBLISH_GITHUB_TOKEN")]
    publish_github_token: Option<String>,
    #[envconfig(from = "FILESYSTEM_COMMIT_AUTHOR_NAME", default = "ObjectiveAI")]
    filesystem_commit_author_name: String,
    #[envconfig(from = "FILESYSTEM_COMMIT_AUTHOR_EMAIL", default = "admin@objective-ai.io")]
    filesystem_commit_author_email: String,
    #[envconfig(from = "MOCK_DELAY_MS", default = "0")]
    mock_delay_ms: u64,
    #[envconfig(from = "MOCK_MAX_TOOL_CALLS", default = "1000")]
    mock_max_tool_calls: u32,
    #[envconfig(from = "ADDRESS", default = "0.0.0.0")]
    address: String,
    #[envconfig(from = "PORT", default = "5000")]
    port: u16,
}

#[tokio::main]
async fn main() {
    // Load .env file if present
    let _ = dotenv::dotenv();

    // Load config from environment
    let Config {
        objectiveai_api_base,
        objectiveai_api_key,
        openrouter_api_base,
        openrouter_api_key,
        claude_agent_sdk,
        user_agent,
        http_referer,
        x_title,
        agent_completions_backoff_current_interval,
        agent_completions_backoff_initial_interval,
        agent_completions_backoff_randomization_factor,
        agent_completions_backoff_multiplier,
        agent_completions_backoff_max_interval,
        agent_completions_backoff_max_elapsed_time,
        mcp_backoff_current_interval,
        mcp_backoff_initial_interval,
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        mcp_backoff_max_interval,
        mcp_backoff_max_elapsed_time,
        github_backoff_current_interval,
        github_backoff_initial_interval,
        github_backoff_randomization_factor,
        github_backoff_multiplier,
        github_backoff_max_interval,
        github_backoff_max_elapsed_time,
        agent_completions_first_chunk_timeout,
        agent_completions_other_chunk_timeout,
        mcp_connect_timeout,
        mcp_call_timeout,
        fetch_github_token,
        publish_github_token,
        filesystem_commit_author_name,
        filesystem_commit_author_email,
        mock_delay_ms,
        mock_max_tool_calls,
        address,
        port,
    } = Config::init_from_env().unwrap();

    // HTTP Client
    let http_client = reqwest::Client::new();

    // ObjectiveAI HTTP Client
    let objectiveai_http_client = Arc::new(objectiveai_http::Client::new(
        http_client.clone(),
        Some(objectiveai_api_base),
        objectiveai_api_key,
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
    ));

    // Vector Completion Votes Fetcher
    let completion_votes_fetcher = Arc::new(
        vector::completions::completion_votes_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ),
    );

    // Vector Cache Vote Fetcher
    let cache_vote_fetcher = Arc::new(
        vector::completions::cache_vote_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ),
    );

    // GitHub Client
    let github_client = Arc::new(github::Client::new(
        http_client.clone(),
        fetch_github_token,
        publish_github_token,
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
        std::time::Duration::from_millis(github_backoff_current_interval),
        std::time::Duration::from_millis(github_backoff_initial_interval),
        github_backoff_randomization_factor,
        github_backoff_multiplier,
        std::time::Duration::from_millis(github_backoff_max_interval),
        std::time::Duration::from_millis(github_backoff_max_elapsed_time),
    ));

    // Filesystem base directory for local function/profile repositories
    let filesystem_base_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".objectiveai")
        .join("functions");

    let filesystem_client = Arc::new(filesystem::Client::new(
        filesystem_base_dir,
        filesystem_commit_author_name,
        filesystem_commit_author_email,
    ));


    // Retrieval: Retrieve Router
    let retrieve_router = Arc::new(retrieval::retrieve::Router::new(
        Arc::new(retrieval::retrieve::github::GithubClient::new(
            github_client.clone(),
        )),
        Arc::new(retrieval::retrieve::filesystem::FilesystemClient::new(
            filesystem_client.clone(),
        )),
        Arc::new(retrieval::retrieve::mock::MockClient),
    ));

    // MCP Client
    let mcp_client = Arc::new(mcp::Client::new(
        http_client.clone(),
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
        std::time::Duration::from_millis(mcp_connect_timeout),
        std::time::Duration::from_millis(
            mcp_backoff_current_interval,
        ),
        std::time::Duration::from_millis(
            mcp_backoff_initial_interval,
        ),
        mcp_backoff_randomization_factor,
        mcp_backoff_multiplier,
        std::time::Duration::from_millis(mcp_backoff_max_interval),
        std::time::Duration::from_millis(
            mcp_backoff_max_elapsed_time,
        ),
        std::time::Duration::from_millis(mcp_call_timeout),
    ));

    // Agent Completions Client
    let agent_completions_client = Arc::new(agent::completions::Client::new(
        mcp_client.clone(),
        retrieve_router.clone(),
        Arc::new(agent::completions::usage_handler::LogUsageHandler),
        Arc::new(agent::completions::openrouter::Client {
            http_client,
            api_base: openrouter_api_base.clone(),
            api_key: openrouter_api_key.clone().unwrap_or_default(),
            user_agent,
            x_title,
            referer: http_referer,
        }),
        Arc::new(agent::completions::claude_agent_sdk::Client::new(None)),
        Arc::new(agent::completions::mock::Client {
            delay: std::time::Duration::from_millis(mock_delay_ms),
            max_tool_calls: mock_max_tool_calls,
        }),
        std::time::Duration::from_millis(
            agent_completions_backoff_current_interval,
        ),
        std::time::Duration::from_millis(
            agent_completions_backoff_initial_interval,
        ),
        agent_completions_backoff_randomization_factor,
        agent_completions_backoff_multiplier,
        std::time::Duration::from_millis(agent_completions_backoff_max_interval),
        std::time::Duration::from_millis(
            agent_completions_backoff_max_elapsed_time,
        ),
        std::time::Duration::from_millis(agent_completions_first_chunk_timeout),
        std::time::Duration::from_millis(agent_completions_other_chunk_timeout),
    ));

    // Vector Completions Client
    let vector_completions_client = Arc::new(vector::completions::Client::new(
        agent_completions_client.clone(),
        retrieve_router.clone(),
        completion_votes_fetcher.clone(),
        cache_vote_fetcher.clone(),
        Arc::new(vector::completions::usage_handler::LogUsageHandler),
    ));

    // Vector Completions Cache Client
    let vector_completions_cache_client =
        Arc::new(vector::completions::cache::Client::new(
            completion_votes_fetcher.clone(),
            cache_vote_fetcher.clone(),
        ));

    // Retrieval: List Router
    let list_router = Arc::new(retrieval::list::Router::new(
        Arc::new(retrieval::list::objectiveai::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
        Arc::new(retrieval::list::filesystem::FilesystemClient::new(
            filesystem_client.clone(),
        )),
        Arc::new(retrieval::list::mock::MockClient),
    ));

    // Retrieval: Usage Router
    let usage_router = Arc::new(retrieval::usage::Router::new(
        Arc::new(retrieval::usage::objectiveai::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
    ));

    // Function Inventions Client
    let function_inventions_client =
        Arc::new(functions::inventions::Client::new(
            agent_completions_client.clone(),
            github_client.clone(),
            filesystem_client.clone(),
            retrieve_router.clone(),
            Arc::new(functions::inventions::usage_handler::LogUsageHandler),
            true, // persist
        ));

    // Function Inventions Recursive Client
    let function_inventions_recursive_client =
        Arc::new(functions::inventions::recursive::Client::new(
            function_inventions_client.clone(),
            Arc::new(
                functions::inventions::recursive::usage_handler::LogUsageHandler,
            ),
        ));

    // Function Executions Client
    let function_executions_client =
        Arc::new(functions::executions::Client::new(
            agent_completions_client.clone(),
            vector_completions_client.clone(),
            retrieve_router.clone(),
            Arc::new(functions::executions::usage_handler::LogUsageHandler),
        ));

    // Functions Profiles Computations Client
    let profile_computations_client =
        Arc::new(functions::profiles::computations::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        ));

    // Auth Client
    let auth_client = Arc::new(auth::ObjectiveAiClient::new(
        objectiveai_http_client.clone(),
    ));


    // Router
    let app = axum::Router::new()
        // Agent Completions - create
        .route(
            "/agent/completions",
            axum::routing::post({
                let agent_completions_client = agent_completions_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::agent::completions::request::AgentCompletionCreateParams,
                >| {
                    create_agent_completion(agent_completions_client, headers, body)
                }
            }),
        )
        // Vector Completions - create
        .route(
            "/vector/completions",
            axum::routing::post({
                let vector_completions_client = vector_completions_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::vector::completions::request::VectorCompletionCreateParams,
                >| {
                    create_vector_completion(vector_completions_client, headers, body)
                }
            }),
        )
        // Vector Completions - get completion votes
        .route(
            "/vector/completions/votes",
            axum::routing::get({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::vector::completions::cache::request::GetCompletionVotesRequest,
                >| {
                    get_vector_completion_votes(
                        vector_completions_cache_client,
                        headers,
                        body,
                    )
                }
            }),
        )
        // Vector Completions - get cache vote
        .route(
            "/vector/completions/cache",
            axum::routing::get({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::vector::completions::cache::request::CacheVoteRequestOwned,
                >| {
                    get_vector_cache_vote(
                        vector_completions_cache_client,
                        headers,
                        body,
                    )
                }
            }),
        )
        // Functions - list
        .route(
            "/functions/list",
            axum::routing::get({
                let list_router = list_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::request::ListFunctionsRequest,
                >| {
                    list_functions(list_router, headers, params)
                }
            }),
        )
        // Functions - get
        .route(
            "/functions",
            axum::routing::get({
                let retrieve_router = retrieve_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::RemotePathCommitOptional,
                >| {
                    get_function(retrieve_router, headers, params)
                }
            }),
        )
        // Functions - get usage
        .route(
            "/functions/usage",
            axum::routing::get({
                let usage_router = usage_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::request::GetFunctionRequest,
                >| {
                    get_function_usage(usage_router, headers, params)
                }
            }),
        )
        // Function Executions - create
        .route(
            "/functions",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::functions::executions::request::FunctionExecutionCreateParams,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        body,
                    )
                }
            }),
        )
        // Function Profiles - list
        .route(
            "/functions/profiles/list",
            axum::routing::get({
                let list_router = list_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::profiles::request::ListProfilesRequest,
                >| {
                    list_profiles(list_router, headers, params)
                }
            }),
        )
        // Function Profiles - get
        .route(
            "/functions/profiles",
            axum::routing::get({
                let retrieve_router = retrieve_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::RemotePathCommitOptional,
                >| {
                    get_profile(retrieve_router, headers, params)
                }
            }),
        )
        // Function Profiles - get usage
        .route(
            "/functions/profiles/usage",
            axum::routing::get({
                let usage_router = usage_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::profiles::request::GetProfileRequest,
                >| {
                    get_profile_usage(usage_router, headers, params)
                }
            }),
        )
        // Function-Profile Pairs - list
        .route(
            "/functions/profiles/pairs/list",
            axum::routing::get({
                let list_router = list_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::request::ListFunctionProfilePairsRequest,
                >| {
                    list_function_profile_pairs(list_router, headers, params)
                }
            }),
        )
        // Function-Profile Pairs - get usage
        .route(
            "/functions/profiles/pairs/usage",
            axum::routing::get({
                let usage_router = usage_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::functions::request::GetFunctionProfilePairUsageRequest,
                >| {
                    get_function_profile_pair_usage(usage_router, headers, params)
                }
            }),
        )
        // Function Inventions - create
        .route(
            "/functions/inventions",
            axum::routing::post({
                let function_inventions_client = function_inventions_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::functions::inventions::request::FunctionInventionCreateParams,
                >| {
                    create_function_invention(function_inventions_client, headers, body)
                }
            }),
        )
        // Function Inventions Recursive - create
        .route(
            "/functions/inventions/recursive",
            axum::routing::post({
                let function_inventions_recursive_client =
                    function_inventions_recursive_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams,
                >| {
                    create_function_invention_recursive(
                        function_inventions_recursive_client,
                        headers,
                        body,
                    )
                }
            }),
        )
        // Function-Profile Pairs - estimate execution cost (no commits)
        .route(
            "/functions/{fowner}/{frepository}/profiles/{powner}/{prepository}/estimate",
            axum::routing::post({
                let pairs_client = pairs_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, powner, prepository)): Path<(String, String, String, String)>,
                      Json(body): Json<
                    objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody,
                >| {
                    estimate_function_profile_pair_cost(
                        pairs_client,
                        headers,
                        fowner,
                        frepository,
                        None,
                        powner,
                        prepository,
                        None,
                        body,
                    )
                }
            }),
        )
        // Function-Profile Pairs - estimate execution cost (fcommit only)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/profiles/{powner}/{prepository}/estimate",
            axum::routing::post({
                let pairs_client = pairs_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, fcommit, powner, prepository)): Path<(String, String, String, String, String)>,
                      Json(body): Json<
                    objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody,
                >| {
                    estimate_function_profile_pair_cost(
                        pairs_client,
                        headers,
                        fowner,
                        frepository,
                        Some(fcommit),
                        powner,
                        prepository,
                        None,
                        body,
                    )
                }
            }),
        )
        // Function-Profile Pairs - estimate execution cost (pcommit only)
        .route(
            "/functions/{fowner}/{frepository}/profiles/{powner}/{prepository}/{pcommit}/estimate",
            axum::routing::post({
                let pairs_client = pairs_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, powner, prepository, pcommit)): Path<(String, String, String, String, String)>,
                      Json(body): Json<
                    objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody,
                >| {
                    estimate_function_profile_pair_cost(
                        pairs_client,
                        headers,
                        fowner,
                        frepository,
                        None,
                        powner,
                        prepository,
                        Some(pcommit),
                        body,
                    )
                }
            }),
        )
        // Function-Profile Pairs - estimate execution cost (both commits)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/profiles/{powner}/{prepository}/{pcommit}/estimate",
            axum::routing::post({
                let pairs_client = pairs_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, fcommit, powner, prepository, pcommit)): Path<(String, String, String, String, String, String)>,
                      Json(body): Json<
                    objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody,
                >| {
                    estimate_function_profile_pair_cost(
                        pairs_client,
                        headers,
                        fowner,
                        frepository,
                        Some(fcommit),
                        powner,
                        prepository,
                        Some(pcommit),
                        body,
                    )
                }
            }),
        )
        // Function Profile Computations - create
        .route(
            "/functions/profiles/compute",
            axum::routing::post({
                let profile_computations_client =
                    profile_computations_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::functions::profiles::computations::request::FunctionProfileComputationCreateParams,
                >| {
                    create_profile_computation(
                        profile_computations_client,
                        headers,
                        body,
                    )
                }
            }),
        )
        // Auth - create API key
        .route(
            "/auth/keys",
            axum::routing::post({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::auth::request::CreateApiKeyRequest,
                >| {
                    create_api_key(auth_client, headers, body)
                }
            }),
        )
        // Auth - create OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::post({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::auth::request::CreateOpenRouterByokApiKeyRequest,
                >| {
                    create_openrouter_byok_api_key(auth_client, headers, body)
                }
            }),
        )
        // Auth - disable API key
        .route(
            "/auth/keys",
            axum::routing::delete({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap, Json(body): Json<
                    objectiveai::auth::request::DisableApiKeyRequest,
                >| {
                    disable_api_key(auth_client, headers, body)
                }
            }),
        )
        // Auth - delete OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::delete({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap| {
                    delete_openrouter_byok_api_key(auth_client, headers)
                }
            }),
        )
        // Auth - list API keys
        .route(
            "/auth/keys",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap| {
                    list_api_keys(auth_client, headers)
                }
            }),
        )
        // Auth - get OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap| {
                    get_openrouter_byok_api_key(auth_client, headers)
                }
            }),
        )
        // Auth - get credits
        .route(
            "/auth/credits",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: axum::http::HeaderMap| {
                    get_credits(auth_client, headers)
                }
            }),
        )
        // Swarm - list
        .route(
            "/swarms/list",
            axum::routing::get({
                let list_router = list_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::swarm::request::ListSwarmsRequest,
                >| {
                    list_swarms(list_router, headers, params)
                }
            }),
        )
        // Swarm - get
        .route(
            "/swarms",
            axum::routing::get({
                let retrieve_router = retrieve_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::RemotePathCommitOptional,
                >| {
                    get_swarm(retrieve_router, headers, params)
                }
            }),
        )
        // Swarm - get usage
        .route(
            "/swarms/usage",
            axum::routing::get({
                let usage_router = usage_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::swarm::request::GetSwarmRequest,
                >| {
                    get_swarm_usage(usage_router, headers, params)
                }
            }),
        )
        // Agent - list
        .route(
            "/agents/list",
            axum::routing::get({
                let list_router = list_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::agent::request::ListAgentsRequest,
                >| {
                    list_agents(list_router, headers, params)
                }
            }),
        )
        // Agent - get
        .route(
            "/agents",
            axum::routing::get({
                let retrieve_router = retrieve_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::RemotePathCommitOptional,
                >| {
                    get_agent(retrieve_router, headers, params)
                }
            }),
        )
        // Agent - get usage
        .route(
            "/agents/usage",
            axum::routing::get({
                let usage_router = usage_router.clone();
                move |headers: axum::http::HeaderMap, Json(params): Json<
                    objectiveai::agent::request::GetAgentRequest,
                >| {
                    get_agent_usage(usage_router, headers, params)
                }
            }),
        )
        // Error - create
        .route(
            "/error",
            axum::routing::post({
                let error_client = Arc::new(objectiveai_api::error::Client::new());
                move |Json(body): Json<
                    objectiveai::error::request::ErrorCreateParams,
                >| {
                    create_error(error_client, body)
                }
            }),
        )
        // CORS
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any)
                .expose_headers(tower_http::cors::Any),
        );

    let listener =
        tokio::net::TcpListener::bind(format!("{}:{}", address, port))
            .await
            .unwrap();

    eprintln!("listening on {}:{}", address, port);
    axum::serve(listener, app).await.unwrap();
}

// Create Context

fn context(headers: &axum::http::HeaderMap) -> ctx::Context<ctx::DefaultContextExt> {
    ctx::Context::new(
        Arc::new(ctx::DefaultContextExt),
        rust_decimal::Decimal::ONE,
        headers,
    )
}

// Agent Completions

async fn create_agent_completion(
    client: Arc<
        agent::completions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::openrouter::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::claude_agent_sdk::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::mock::Agent,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::agent::completions::request::AgentCompletionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(
                ctx,
                Arc::new(body),
                None,
                None,
                None,
                None,
            )
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .filter_map(|item| {
                        match item {
                            agent::completions::StreamItem::Chunk(chunk) => {
                                Some(Ok::<Event, Infallible>(
                                    Event::default()
                                        .data(serde_json::to_string(&chunk).unwrap()),
                                ))
                            }
                            agent::completions::StreamItem::State(_) => None,
                        }
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(
                ctx,
                Arc::new(body),
                None,
                None,
                None,
                None,
            )
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Vector Completions

async fn create_vector_completion(
    client: Arc<
        vector::completions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::openrouter::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::claude_agent_sdk::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::mock::Agent,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::vector::completions::request::VectorCompletionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client.create_unary_handle_usage(ctx, Arc::new(body)).await {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Functions

async fn list_functions(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::functions::request::ListFunctionsRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    let source = params.source.map(|s| match s {
        objectiveai::functions::request::ListFunctionsSource::All => retrieval::list::SourceFilter::All,
        objectiveai::functions::request::ListFunctionsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai::functions::request::ListFunctionsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai::functions::request::ListFunctionsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_functions(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_function_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::functions::request::GetFunctionRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match usage_router.get_function_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn execute_function(
    client: Arc<
        functions::executions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::openrouter::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::claude_agent_sdk::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::mock::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::executions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    request: objectiveai::functions::executions::request::FunctionExecutionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if request.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(request))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(ctx, Arc::new(request))
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Profiles

async fn list_profiles(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::functions::profiles::request::ListProfilesRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    let source = params.source.map(|s| match s {
        objectiveai::functions::profiles::request::ListProfilesSource::All => retrieval::list::SourceFilter::All,
        objectiveai::functions::profiles::request::ListProfilesSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai::functions::profiles::request::ListProfilesSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai::functions::profiles::request::ListProfilesSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_profiles(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_profile_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::functions::profiles::request::GetProfileRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match usage_router.get_profile_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Function-Profile Pairs

async fn list_function_profile_pairs(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    _params: objectiveai::functions::request::ListFunctionProfilePairsRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match list_router.list_function_profile_pairs(&ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_function_profile_pair_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::functions::request::GetFunctionProfilePairUsageRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match usage_router.get_function_profile_pair_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn estimate_function_profile_pair_cost(
    client: Arc<
        impl functions::pair_retrieval_client::Client<ctx::DefaultContextExt>
        + Send
        + Sync
        + 'static,
    >,
    headers: HeaderMap,
    fowner: String,
    frepository: String,
    fcommit: Option<String>,
    powner: String,
    prepository: String,
    pcommit: Option<String>,
    body: objectiveai_api::functions::executions::cost_estimate::CostEstimateRequestBody,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .get_function_profile_pair_usage(
            ctx,
            &fowner,
            &frepository,
            fcommit.as_deref(),
            &powner,
            &prepository,
            pcommit.as_deref(),
        )
        .await
    {
        Ok(usage) => {
            let (input_size, estimate) =
                objectiveai_api::functions::executions::cost_estimate::estimate_cost(
                    &usage,
                    &body.input,
                );

            let response =
                objectiveai_api::functions::executions::cost_estimate::CostEstimateResponse {
                    function_owner: fowner,
                    function_repository: frepository,
                    function_commit: fcommit,
                    profile_owner: powner,
                    profile_repository: prepository,
                    profile_commit: pcommit,
                    input_size,
                    usage,
                    estimate,
                };

            Json(response).into_response()
        }
        Err(e) => e.into_response(),
    }
}

// Vector Completions Cache

async fn get_vector_completion_votes(
    client: Arc<
        vector::completions::cache::Client<
            ctx::DefaultContextExt,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::vector::completions::cache::request::GetCompletionVotesRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.fetch_completion_votes(ctx, &body.id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_vector_cache_vote(
    client: Arc<
        vector::completions::cache::Client<
            ctx::DefaultContextExt,
            impl vector::completions::completion_votes_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl vector::completions::cache_vote_fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::vector::completions::cache::request::CacheVoteRequestOwned,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .fetch_cache_vote(
            ctx,
            &body.agent,
            &body.messages,
            &body.responses,
        )
        .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Functions - get

async fn get_function(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers);
    match retrieve_router.endpoint_get_function(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Profiles - get

async fn get_profile(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers);
    match retrieve_router.endpoint_get_profile(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Profile Computations

async fn create_profile_computation(
    // client: Arc<
    //     impl functions::profiles::computations::Client<ctx::DefaultContextExt>
    //     + Send
    //     + Sync
    //     + 'static,
    // >,
    // https://github.com/rust-lang/rust/issues/100013
    // using a concrete type for client instead
    client: Arc<functions::profiles::computations::ObjectiveAiClient>,
    headers: axum::http::HeaderMap,
    request: objectiveai::functions::profiles::computations::request::FunctionProfileComputationCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if request.stream.unwrap_or(false) {
        match client.create_streaming(ctx, Arc::new(request)).await {
            Ok(stream) => Sse::new(
                stream
                    .map(|result| {
                        Ok::<Event, Infallible>(
                            Event::default().data(
                                match result {
                                    Ok(chunk) => serde_json::to_string(&chunk),
                                    Err(e) => serde_json::to_string(&e),
                                }
                                .unwrap(),
                            ),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        match client.create_unary(ctx, Arc::new(request)).await {
            Ok(r) => Json(r).into_response(),
            Err(e) => e.into_response(),
        }
    }
}

// Auth

async fn create_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::auth::request::CreateApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.create_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::auth::request::CreateOpenRouterByokApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.create_openrouter_byok_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn disable_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::auth::request::DisableApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.disable_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.delete_openrouter_byok_api_key(ctx).await {
        Ok(()) => axum::http::StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn list_api_keys(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list_api_keys(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_openrouter_byok_api_key(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_openrouter_byok_api_key(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_credits(
    client: Arc<
        impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static,
    >,
    headers: axum::http::HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_credits(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Swarm

async fn list_swarms(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::swarm::request::ListSwarmsRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    let source = params.source.map(|s| match s {
        objectiveai::swarm::request::ListSwarmsSource::All => retrieval::list::SourceFilter::All,
        objectiveai::swarm::request::ListSwarmsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai::swarm::request::ListSwarmsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai::swarm::request::ListSwarmsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_swarms(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_swarm(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers);
    match retrieve_router.endpoint_get_swarm(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_swarm_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::swarm::request::GetSwarmRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match usage_router.get_swarm_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Agent

async fn list_agents(
    list_router: Arc<ListRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::agent::request::ListAgentsRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    let source = params.source.map(|s| match s {
        objectiveai::agent::request::ListAgentsSource::All => retrieval::list::SourceFilter::All,
        objectiveai::agent::request::ListAgentsSource::Mock => retrieval::list::SourceFilter::Mock,
        objectiveai::agent::request::ListAgentsSource::Filesystem => retrieval::list::SourceFilter::Filesystem,
        objectiveai::agent::request::ListAgentsSource::Objectiveai => retrieval::list::SourceFilter::Objectiveai,
    });
    match list_router.list_agents(&ctx, source).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_agent(
    retrieve_router: Arc<RetrieveRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::RemotePathCommitOptional,
) -> axum::response::Response {
    let ctx = context(&headers);
    match retrieve_router.endpoint_get_agent(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_agent_usage(
    usage_router: Arc<UsageRouter>,
    headers: axum::http::HeaderMap,
    params: objectiveai::agent::request::GetAgentRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match usage_router.get_agent_usage(&ctx, &params).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}
// Function Inventions

async fn create_function_invention(
    client: Arc<
        functions::inventions::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::openrouter::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::claude_agent_sdk::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::mock::Agent,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl functions::inventions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::functions::inventions::request::FunctionInventionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Function Inventions Recursive

async fn create_function_invention_recursive(
    client: Arc<
        functions::inventions::recursive::Client<
            ctx::DefaultContextExt,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::openrouter::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::claude_agent_sdk::Agent,
            > + Send
            + Sync
            + 'static,
            impl agent::completions::UpstreamClient<
                objectiveai::agent::mock::Agent,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl agent::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl functions::inventions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl retrieval::retrieve::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::inventions::recursive::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: axum::http::HeaderMap,
    body: objectiveai::functions::inventions::recursive::request::FunctionInventionRecursiveCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|chunk| {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(serde_json::to_string(&chunk).unwrap()),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(r) => Json(r).into_response(),
            Err(e) => ResponseError::from(&e).into_response(),
        }
    }
}

// Error

async fn create_error(
    client: Arc<objectiveai_api::error::Client>,
    body: objectiveai::error::request::ErrorCreateParams,
) -> axum::response::Response {
    if body.stream.unwrap_or(false) {
        match client.create_streaming(&body) {
            Ok(stream) => Sse::new(
                stream
                    .map(|result| {
                        Ok::<Event, Infallible>(
                            Event::default().data(
                                match result {
                                    Ok(chunk) => serde_json::to_string(&chunk),
                                    Err(e) => serde_json::to_string(&e),
                                }
                                .unwrap(),
                            ),
                        )
                    })
                    .chain(StreamOnce::new(
                        Ok(Event::default().data("[DONE]")),
                    )),
            )
            .into_response(),
            Err(e) => e.into_response(),
        }
    } else {
        match client.create_unary(&body) {
            Ok(r) => Json(r).into_response(),
            Err(e) => e.into_response(),
        }
    }
}
