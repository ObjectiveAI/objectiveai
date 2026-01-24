//! ObjectiveAI API server.
//!
//! REST API server for chat completions, vector completions, Functions,
//! Profiles, Ensembles, and authentication.

use axum::{
    Json,
    extract::Path,
    http::HeaderMap,
    response::{IntoResponse, Sse, sse::Event},
};
use envconfig::Envconfig;
use objectiveai::error::ResponseError;
use objectiveai_api::{
    auth, chat, ctx, ensemble, ensemble_llm,
    error::ResponseErrorExt,
    functions::{self, profiles::computations::Client},
    util::StreamOnce,
    vector,
};
use std::{convert::Infallible, sync::Arc};
use tokio_stream::StreamExt;

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
    openrouter_api_key: String,
    #[envconfig(from = "USER_AGENT")]
    user_agent: Option<String>,
    #[envconfig(from = "HTTP_REFERER")]
    http_referer: Option<String>,
    #[envconfig(from = "X_TITLE")]
    x_title: Option<String>,
    #[envconfig(
        from = "CHAT_COMPLETIONS_BACKOFF_CURRENT_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    chat_completions_backoff_current_interval: u64,
    #[envconfig(
        from = "CHAT_COMPLETIONS_BACKOFF_INITIAL_INTERVAL",
        default = "100" // 100 milliseconds
    )]
    chat_completions_backoff_initial_interval: u64,
    #[envconfig(
        from = "CHAT_COMPLETIONS_BACKOFF_RANDOMIZATION_FACTOR",
        default = "0.5"
    )]
    chat_completions_backoff_randomization_factor: f64,
    #[envconfig(from = "CHAT_COMPLETIONS_BACKOFF_MULTIPLIER", default = "1.5")]
    chat_completions_backoff_multiplier: f64,
    #[envconfig(
        from = "CHAT_COMPLETIONS_BACKOFF_MAX_INTERVAL",
        default = "1000" // 1 second
    )]
    chat_completions_backoff_max_interval: u64,
    #[envconfig(
        from = "CHAT_COMPLETIONS_BACKOFF_MAX_ELAPSED_TIME",
        default = "40000" // 40 seconds
    )]
    chat_completions_backoff_max_elapsed_time: u64,
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
        user_agent,
        http_referer,
        x_title,
        chat_completions_backoff_current_interval,
        chat_completions_backoff_initial_interval,
        chat_completions_backoff_randomization_factor,
        chat_completions_backoff_multiplier,
        chat_completions_backoff_max_interval,
        chat_completions_backoff_max_elapsed_time,
        address,
        port,
    } = Config::init_from_env().unwrap();

    // HTTP Client
    let http_client = reqwest::Client::new();

    // ObjectiveAI HTTP Client
    let objectiveai_http_client = Arc::new(objectiveai::HttpClient::new(
        http_client.clone(),
        Some(objectiveai_api_base),
        objectiveai_api_key,
        user_agent.clone(),
        x_title.clone(),
        http_referer.clone(),
    ));

    // Ensemble LLM Fetcher
    let ensemble_llm_fetcher = Arc::new(
        ensemble_llm::fetcher::CachingFetcher::new(Arc::new(
            ensemble_llm::fetcher::ObjectiveAiFetcher::new(
                objectiveai_http_client.clone(),
            ),
        )),
    );

    // Chat Completions Client
    let chat_completions_client = Arc::new(chat::completions::Client::<
        ctx::DefaultContextExt,
        _,
        _,
    >::new(
        ensemble_llm_fetcher.clone(),
        Arc::new(chat::completions::usage_handler::LogUsageHandler),
        chat::completions::upstream::Client::new(
            chat::completions::upstream::openrouter::Client::new(
                http_client,
                openrouter_api_base,
                openrouter_api_key,
                user_agent,
                x_title,
                http_referer,
            ),
        ),
        std::time::Duration::from_millis(
            chat_completions_backoff_current_interval,
        ),
        std::time::Duration::from_millis(
            chat_completions_backoff_initial_interval,
        ),
        chat_completions_backoff_randomization_factor,
        chat_completions_backoff_multiplier,
        std::time::Duration::from_millis(chat_completions_backoff_max_interval),
        std::time::Duration::from_millis(
            chat_completions_backoff_max_elapsed_time,
        ),
    ));

    // Ensemble Fetcher
    let ensemble_fetcher = Arc::new(
        ensemble::fetcher::CachingFetcher::new(Arc::new(
            ensemble::fetcher::ObjectiveAiFetcher::new(
                objectiveai_http_client.clone(),
            ),
        )),
    );

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

    // Vector Completions Client
    let vector_completions_client = Arc::new(vector::completions::Client::new(
        chat_completions_client.clone(),
        ensemble_fetcher.clone(),
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

    // Function Fetcher
    let function_fetcher =
        Arc::new(functions::function_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ));

    // Function Profile Fetcher
    let profile_fetcher =
        Arc::new(functions::profile_fetcher::ObjectiveAiFetcher::new(
            objectiveai_http_client.clone(),
        ));

    // Function Executions Client
    let function_executions_client =
        Arc::new(functions::executions::Client::new(
            chat_completions_client.clone(),
            ensemble_fetcher.clone(),
            vector_completions_client.clone(),
            function_fetcher.clone(),
            profile_fetcher.clone(),
            Arc::new(functions::executions::usage_handler::LogUsageHandler),
        ));

    // Functions Profiles Computations Client
    let profile_computations_client =
        Arc::new(functions::profiles::computations::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        ));

    // Functions Client
    let functions_client = Arc::new(functions::Client::new(
        function_fetcher.clone(),
        Arc::new(functions::retrieval_client::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
    ));

    // Function Profiles Client
    let profiles_client = Arc::new(functions::profiles::Client::new(
        profile_fetcher.clone(),
        Arc::new(
            functions::profiles::retrieval_client::ObjectiveAiClient::new(
                objectiveai_http_client.clone(),
            ),
        ),
    ));

    // Auth Client
    let auth_client = Arc::new(auth::ObjectiveAiClient::new(
        objectiveai_http_client.clone(),
    ));

    // Ensemble Client
    let ensemble_client = Arc::new(ensemble::Client::new(
        ensemble_fetcher.clone(),
        Arc::new(ensemble::retrieval_client::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
    ));

    // Ensemble LLM Client
    let ensemble_llm_client = Arc::new(ensemble_llm::Client::new(
        ensemble_llm_fetcher.clone(),
        Arc::new(ensemble_llm::retrieval_client::ObjectiveAiClient::new(
            objectiveai_http_client.clone(),
        )),
    ));

    // Router
    let app = axum::Router::new()
        // Chat Completions - create
        .route(
            "/chat/completions",
            axum::routing::post({
                let chat_completions_client = chat_completions_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
                    objectiveai::chat::completions::request::ChatCompletionCreateParams,
                >| {
                    create_chat_completion(chat_completions_client, headers, body)
                }
            }),
        )
        // Vector Completions - create
        .route(
            "/vector/completions",
            axum::routing::post({
                let vector_completions_client = vector_completions_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
                    objectiveai::vector::completions::request::VectorCompletionCreateParams,
                >| {
                    create_vector_completion(vector_completions_client, headers, body)
                }
            }),
        )
        // Vector Completions - get completion votes
        .route(
            "/vector/completions/{id}",
            axum::routing::post({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                move |headers: HeaderMap,
                      Path(id): Path<String>| {
                    get_vector_completion_votes(
                        vector_completions_cache_client,
                        headers,
                        id,
                    )
                }
            }),
        )
        // Vector Completions - get cache vote
        .route(
            "/vector/completions/cache",
            axum::routing::post({
                let vector_completions_cache_client =
                    vector_completions_cache_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
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
            "/functions",
            axum::routing::get({
                let functions_client = functions_client.clone();
                move |headers: HeaderMap| list_functions(functions_client, headers)
            }),
        )
        // Functions - get (without commit)
        .route(
            "/functions/{fowner}/{frepository}",
            axum::routing::get({
                let functions_client = functions_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository)): Path<(String, String)>| {
                    get_function(functions_client, headers, fowner, frepository, None)
                }
            }),
        )
        // Functions - get (with commit)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}",
            axum::routing::get({
                let functions_client = functions_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, fcommit)): Path<(
                    String,
                    String,
                    String,
                )>| {
                    get_function(
                        functions_client,
                        headers,
                        fowner,
                        frepository,
                        Some(fcommit),
                    )
                }
            }),
        )
        // Functions - get usage (without commit)
        .route(
            "/functions/{fowner}/{frepository}/usage",
            axum::routing::get({
                let functions_client = functions_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository)): Path<(String, String)>| {
                    get_function_usage(functions_client, headers, fowner, frepository, None)
                }
            }),
        )
        // Functions - get usage (with commit)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/usage",
            axum::routing::get({
                let functions_client = functions_client.clone();
                move |headers: HeaderMap,
                      Path((fowner, frepository, fcommit)): Path<(String, String, String)>| {
                    get_function_usage(
                        functions_client,
                        headers,
                        fowner,
                        frepository,
                        Some(fcommit),
                    )
                }
            }),
        )
        // Function Executions - create
        // inline function
        // inline profile
        .route(
            "/functions",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionInlineProfileInlineRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionInlineProfileInline {
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (without commit)
        // inline profile
        .route(
            "/functions/{fowner}/{frepository}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileInlineRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileInlineRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileInline {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (with commit)
        // inline profile
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileInlineRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileInlineRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileInline {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // inline function
        // remote profile (without commit)
        .route(
            "/functions/profiles/{powner}/{prepository}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionInlineProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionInlineProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionInlineProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // inline function
        // remote profile (with commit)
        .route(
            "/functions/profiles/{powner}/{prepository}/{pcommit}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionInlineProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionInlineProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionInlineProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (without commit)
        // remote profile (without commit)
        .route(
            "/functions/{fowner}/{frepository}/profiles/{powner}/{prepository}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (without commit)
        // remote profile (with commit)
        .route(
            "/functions/{fowner}/{frepository}/profiles/{powner}/{prepository}/{pcommit}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (with commit)
        // remote profile (without commit)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/profiles/{powner}/{prepository}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Executions - create
        // remote function (with commit)
        // remote profile (with commit)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/profiles/{powner}/{prepository}/{pcommit}",
            axum::routing::post({
                let function_executions_client = function_executions_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::executions::request::FunctionRemoteProfileRemoteRequestBody,
                >| {
                    execute_function(
                        function_executions_client,
                        headers,
                        objectiveai::functions::executions::request::Request::FunctionRemoteProfileRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Profiles - list
        .route(
            "/functions/profiles",
            axum::routing::get({
                let profiles_client = profiles_client.clone();
                move |headers: HeaderMap| list_profiles(profiles_client, headers)
            }),
        )
        // Function Profiles - get (without commit)
        .route(
            "/functions/profiles/{powner}/{prepository}",
            axum::routing::get({
                let profiles_client = profiles_client.clone();
                move |headers: HeaderMap,
                      Path((powner, prepository)): Path<(String, String)>| {
                    get_profile(profiles_client, headers, powner, prepository, None)
                }
            }),
        )
        // Function Profiles - get (with commit)
        .route(
            "/functions/profiles/{powner}/{prepository}/{pcommit}",
            axum::routing::get({
                let profiles_client = profiles_client.clone();
                move |headers: HeaderMap,
                      Path((powner, prepository, pcommit)): Path<(
                    String,
                    String,
                    String,
                )>| {
                    get_profile(
                        profiles_client,
                        headers,
                        powner,
                        prepository,
                        Some(pcommit),
                    )
                }
            }),
        )
        // Function Profiles - get usage (without commit)
        .route(
            "/functions/profiles/{powner}/{prepository}/usage",
            axum::routing::get({
                let profiles_client = profiles_client.clone();
                move |headers: HeaderMap,
                      Path((powner, prepository)): Path<(String, String)>| {
                    get_profile_usage(profiles_client, headers, powner, prepository, None)
                }
            }),
        )
        // Function Profiles - get usage (with commit)
        .route(
            "/functions/profiles/{powner}/{prepository}/{pcommit}/usage",
            axum::routing::get({
                let profiles_client = profiles_client.clone();
                move |headers: HeaderMap,
                      Path((powner, prepository, pcommit)): Path<(String, String, String)>| {
                    get_profile_usage(
                        profiles_client,
                        headers,
                        powner,
                        prepository,
                        Some(pcommit),
                    )
                }
            }),
        )
        // Function Profile Computations - create
        // inline function
        .route(
            "/functions/profiles/compute",
            axum::routing::post({
                let profile_computations_client =
                    profile_computations_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
                    objectiveai::functions::profiles::computations::request::FunctionInlineRequestBody,
                >| {
                    create_profile_computation(
                        profile_computations_client,
                        headers,
                        objectiveai::functions::profiles::computations::request::Request::FunctionInline {
                            body,
                        },
                    )
                }
            }),
        )
        // Function Profile Computations - create
        // remote function (without commit)
        .route(
            "/functions/{fowner}/{frepository}/profiles/compute",
            axum::routing::post({
                let profile_computations_client =
                    profile_computations_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::profiles::computations::request::FunctionRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::profiles::computations::request::FunctionRemoteRequestBody,
                >| {
                    create_profile_computation(
                        profile_computations_client,
                        headers,
                        objectiveai::functions::profiles::computations::request::Request::FunctionRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Function Profile Computations - create
        // remote function (with commit)
        .route(
            "/functions/{fowner}/{frepository}/{fcommit}/profiles/compute",
            axum::routing::post({
                let profile_computations_client =
                    profile_computations_client.clone();
                move |headers: HeaderMap,
                      Path(path): Path<
                    objectiveai::functions::profiles::computations::request::FunctionRemoteRequestPath,
                >,
                      Json(body): Json<
                    objectiveai::functions::profiles::computations::request::FunctionRemoteRequestBody,
                >| {
                    create_profile_computation(
                        profile_computations_client,
                        headers,
                        objectiveai::functions::profiles::computations::request::Request::FunctionRemote {
                            path,
                            body,
                        },
                    )
                }
            }),
        )
        // Auth - create API key
        .route(
            "/auth/keys",
            axum::routing::post({
                let auth_client = auth_client.clone();
                move |headers: HeaderMap,
                      Json(body): Json<
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
                move |headers: HeaderMap,
                      Json(body): Json<
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
                move |headers: HeaderMap,
                      Json(body): Json<
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
                move |headers: HeaderMap| {
                    delete_openrouter_byok_api_key(auth_client, headers)
                }
            }),
        )
        // Auth - list API keys
        .route(
            "/auth/keys",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: HeaderMap| {
                    list_api_keys(auth_client, headers)
                }
            }),
        )
        // Auth - get OpenRouter BYOK API key
        .route(
            "/auth/keys/openrouter",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: HeaderMap| {
                    get_openrouter_byok_api_key(auth_client, headers)
                }
            }),
        )
        // Auth - get credits
        .route(
            "/auth/credits",
            axum::routing::get({
                let auth_client = auth_client.clone();
                move |headers: HeaderMap| {
                    get_credits(auth_client, headers)
                }
            }),
        )
        // Ensemble - list
        .route(
            "/ensembles",
            axum::routing::get({
                let ensemble_client = ensemble_client.clone();
                move |headers: HeaderMap| {
                    list_ensembles(ensemble_client, headers)
                }
            }),
        )
        // Ensemble - get
        .route(
            "/ensembles/{id}",
            axum::routing::get({
                let ensemble_client = ensemble_client.clone();
                move |headers: HeaderMap, Path(id): Path<String>| {
                    get_ensemble(ensemble_client, headers, id)
                }
            }),
        )
        // Ensemble - get usage
        .route(
            "/ensembles/{id}/usage",
            axum::routing::get({
                let ensemble_client = ensemble_client.clone();
                move |headers: HeaderMap, Path(id): Path<String>| {
                    get_ensemble_usage(ensemble_client, headers, id)
                }
            }),
        )
        // Ensemble LLM - list
        .route(
            "/ensemble_llms",
            axum::routing::get({
                let ensemble_llm_client = ensemble_llm_client.clone();
                move |headers: HeaderMap| {
                    list_ensemble_llms(ensemble_llm_client, headers)
                }
            }),
        )
        // Ensemble LLM - get
        .route(
            "/ensemble_llms/{id}",
            axum::routing::get({
                let ensemble_llm_client = ensemble_llm_client.clone();
                move |headers: HeaderMap, Path(id): Path<String>| {
                    get_ensemble_llm(ensemble_llm_client, headers, id)
                }
            }),
        )
        // Ensemble LLM - get usage
        .route(
            "/ensemble_llms/{id}/usage",
            axum::routing::get({
                let ensemble_llm_client = ensemble_llm_client.clone();
                move |headers: HeaderMap, Path(id): Path<String>| {
                    get_ensemble_llm_usage(ensemble_llm_client, headers, id)
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

    axum::serve(listener, app).await.unwrap();
}

// Create Context

fn context(headers: &HeaderMap) -> ctx::Context<ctx::DefaultContextExt> {
    ctx::Context::new(
        Arc::new(ctx::DefaultContextExt::from_headers(headers)),
        rust_decimal::Decimal::ONE,
    )
}

// Chat Completions

async fn create_chat_completion(
    client: Arc<
        chat::completions::Client<
            ctx::DefaultContextExt,
            impl ensemble_llm::fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl chat::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
    body: objectiveai::chat::completions::request::ChatCompletionCreateParams,
) -> axum::response::Response {
    let ctx = context(&headers);
    if body.stream.unwrap_or(false) {
        match client
            .create_streaming_for_chat_handle_usage(ctx, Arc::new(body))
            .await
        {
            Ok(stream) => Sse::new(
                stream
                    .map(|result| {
                        Ok::<Event, Infallible>(
                            Event::default().data(
                                match result {
                                    Ok(chunk) => serde_json::to_string(&chunk),
                                    Err(e) => serde_json::to_string(
                                        &ResponseError::from(&e),
                                    ),
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
            Err(e) => ResponseError::from(&e).into_response(),
        }
    } else {
        match client
            .create_unary_for_chat_handle_usage(ctx, Arc::new(body))
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
            impl ensemble_llm::fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl chat::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl ensemble::fetcher::Fetcher<
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
    headers: HeaderMap,
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
    client: Arc<
        functions::Client<
            ctx::DefaultContextExt,
            impl functions::function_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::retrieval_client::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list_functions(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => ResponseError::from(&e).into_response(),
    }
}

async fn get_function_usage(
    client: Arc<
        functions::Client<
            ctx::DefaultContextExt,
            impl functions::function_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::retrieval_client::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
    owner: String,
    repository: String,
    commit: Option<String>,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .get_function_usage(ctx, &owner, &repository, commit.as_deref())
        .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => ResponseError::from(&e).into_response(),
    }
}

async fn execute_function(
    client: Arc<
        functions::executions::Client<
            ctx::DefaultContextExt,
            impl ensemble_llm::fetcher::Fetcher<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl chat::completions::usage_handler::UsageHandler<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
            impl ensemble::fetcher::Fetcher<
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
            impl functions::function_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::profile_fetcher::Fetcher<ctx::DefaultContextExt>
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
    headers: HeaderMap,
    request: objectiveai::functions::executions::request::Request,
) -> axum::response::Response {
    let ctx = context(&headers);
    if request.base().stream.unwrap_or(false) {
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
    client: Arc<
        functions::profiles::Client<
            ctx::DefaultContextExt,
            impl functions::profile_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::profiles::retrieval_client::Client<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list_profiles(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => ResponseError::from(&e).into_response(),
    }
}

async fn get_profile_usage(
    client: Arc<
        functions::profiles::Client<
            ctx::DefaultContextExt,
            impl functions::profile_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::profiles::retrieval_client::Client<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
    owner: String,
    repository: String,
    commit: Option<String>,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .get_profile_usage(ctx, &owner, &repository, commit.as_deref())
        .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => ResponseError::from(&e).into_response(),
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
    headers: HeaderMap,
    id: String,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.fetch_completion_votes(ctx, &id).await {
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
    headers: HeaderMap,
    body: objectiveai::vector::completions::cache::request::CacheVoteRequestOwned,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .fetch_cache_vote(
            ctx,
            &body.model,
            body.models.as_deref(),
            &body.messages,
            body.tools.as_deref(),
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
    client: Arc<
        functions::Client<
            ctx::DefaultContextExt,
            impl functions::function_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::retrieval_client::Client<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
    owner: String,
    repository: String,
    commit: Option<String>,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .get_function(ctx, &owner, &repository, commit.as_deref())
        .await
    {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Profiles - get

async fn get_profile(
    client: Arc<
        functions::profiles::Client<
            ctx::DefaultContextExt,
            impl functions::profile_fetcher::Fetcher<ctx::DefaultContextExt>
            + Send
            + Sync
            + 'static,
            impl functions::profiles::retrieval_client::Client<
                ctx::DefaultContextExt,
            > + Send
            + Sync
            + 'static,
        >,
    >,
    headers: HeaderMap,
    owner: String,
    repository: String,
    commit: Option<String>,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client
        .get_profile(ctx, &owner, &repository, commit.as_deref())
        .await
    {
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
    headers: HeaderMap,
    request: objectiveai::functions::profiles::computations::request::Request,
) -> axum::response::Response {
    let ctx = context(&headers);
    if request.base().stream.unwrap_or(false) {
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
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
    body: objectiveai::auth::request::CreateApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.create_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn create_openrouter_byok_api_key(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
    body: objectiveai::auth::request::CreateOpenRouterByokApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.create_openrouter_byok_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn disable_api_key(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
    body: objectiveai::auth::request::DisableApiKeyRequest,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.disable_api_key(ctx, body).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn delete_openrouter_byok_api_key(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.delete_openrouter_byok_api_key(ctx).await {
        Ok(()) => axum::http::StatusCode::OK.into_response(),
        Err(e) => e.into_response(),
    }
}

async fn list_api_keys(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list_api_keys(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_openrouter_byok_api_key(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_openrouter_byok_api_key(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_credits(
    client: Arc<impl auth::Client<ctx::DefaultContextExt> + Send + Sync + 'static>,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_credits(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Ensemble

async fn list_ensembles(
    client: Arc<
        ensemble::Client<
            ctx::DefaultContextExt,
            impl ensemble::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_ensemble(
    client: Arc<
        ensemble::Client<
            ctx::DefaultContextExt,
            impl ensemble::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
    id: String,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get(ctx, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_ensemble_usage(
    client: Arc<
        ensemble::Client<
            ctx::DefaultContextExt,
            impl ensemble::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
    id: String,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_usage(ctx, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

// Ensemble LLM

async fn list_ensemble_llms(
    client: Arc<
        ensemble_llm::Client<
            ctx::DefaultContextExt,
            impl ensemble_llm::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble_llm::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.list(ctx).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_ensemble_llm(
    client: Arc<
        ensemble_llm::Client<
            ctx::DefaultContextExt,
            impl ensemble_llm::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble_llm::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
    id: String,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get(ctx, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}

async fn get_ensemble_llm_usage(
    client: Arc<
        ensemble_llm::Client<
            ctx::DefaultContextExt,
            impl ensemble_llm::fetcher::Fetcher<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
            impl ensemble_llm::retrieval_client::Client<ctx::DefaultContextExt>
                + Send
                + Sync
                + 'static,
        >,
    >,
    headers: HeaderMap,
    id: String,
) -> axum::response::Response {
    let ctx = context(&headers);
    match client.get_usage(ctx, &id).await {
        Ok(r) => Json(r).into_response(),
        Err(e) => e.into_response(),
    }
}
