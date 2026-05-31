use crate::{ctx, util::StreamOnce};
use futures::{Stream, StreamExt};
use objectiveai_sdk::error::StatusError;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time,
};

type FunctionInventionChunk =
    objectiveai_sdk::functions::inventions::response::streaming::FunctionInventionChunk;
type InventionAgentCompletionChunk =
    objectiveai_sdk::functions::inventions::response::streaming::AgentCompletionChunk;
type Object = objectiveai_sdk::functions::inventions::response::streaming::Object;
type Params = objectiveai_sdk::functions::inventions::Params;
type State = objectiveai_sdk::functions::inventions::State;

use objectiveai_sdk::functions::inventions::InventionState;

/// Generates a unique response ID for Function inventions.
pub fn invention_response_id(created: u64) -> String {
    crate::util::response_id(Some("fninv1"), created)
}

/// Maximum total name length in bytes.
const MAX_NAME_LEN: usize = 100;
/// Maximum name length when no valid path segment exists.
/// Leaves room for `-` (1 byte) + base62 path segment (up to 22 bytes).
const MAX_NAME_LEN_WITHOUT_PATH: usize = 77;

/// Map the integer `invention_step` (which `run_step` already takes as
/// `usize`) to a stable wire-name for error reporting.
const fn step_name(invention_step: usize) -> &'static str {
    match invention_step {
        0 => "essay",
        1 => "input_schema",
        2 => "essay_tasks",
        3 => "tasks",
        4 => "description",
        _ => "unknown",
    }
}

/// Validates the invention name length constraints.
///
/// - Must be at most 100 bytes total.
/// - If the name does not already contain a valid base62 path segment
///   (the part after the last `-`), it must be at most 77 bytes to leave
///   room for child path segments (`-` + up to 22 bytes of base62).
fn validate_name(name: &str) -> Result<(), super::Error> {
    let len = name.len();
    if len > MAX_NAME_LEN {
        return Err(super::Error::InvalidName(format!(
            "name is {} bytes, maximum is {}",
            len, MAX_NAME_LEN,
        )));
    }
    let has_valid_path = name
        .rsplit_once('-')
        .and_then(|(_, last)| objectiveai_sdk::functions::inventions::path::b62_to_path::<u64>(last).ok())
        .is_some();
    if !has_valid_path && len > MAX_NAME_LEN_WITHOUT_PATH {
        return Err(super::Error::InvalidName(format!(
            "name is {} bytes without a path segment, maximum is {} \
             (must leave room for child path `-` + up to 22 bytes)",
            len, MAX_NAME_LEN_WITHOUT_PATH,
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tool subscription helpers
// ---------------------------------------------------------------------------

/// Prefix the proxy stamps onto every tool name from the InventionServer
/// upstream. Either `oaifi_<name>` (single upstream) or
/// `oaifi_<digits>_<name>` (proxy collision-disambiguated when the
/// same prefix would otherwise collide with another upstream). The
/// prefix string itself is the InventionServer's rmcp
/// `server_info.name` — see `invention_server.rs::InventionMcp::get_info`.
const PROXY_INVENTION_PREFIX: &str = "oaifi_";

/// Wait until every name in `expected` is present in the agent's MCP
/// view of available tools, observing through the agent's existing MCP
/// connection from the previous step's continuation.
///
/// First reads `list_tools` once with no clock running (the user's
/// "free read" semantics). If the expected set is already covered,
/// returns immediately. Otherwise enters a `subscribe_tools` loop under
/// a single shared clock that starts on the first subscribe call. The
/// loop exits with `Error::ToolSubscriptionTimeout` when the budget is
/// exhausted without coverage.
async fn wait_for_tools_visible(
    observer: &objectiveai_sdk::mcp::Connection,
    expected: &[String],
    overall_timeout: time::Duration,
) -> Result<(), super::Error> {
    let mut current = observer
        .list_tools()
        .await
        .map_err(|e| super::Error::McpListTools(format!("{e}")))?;
    if all_present(&current, expected) {
        return Ok(());
    }

    let started = time::Instant::now();
    loop {
        let elapsed = started.elapsed();
        let remaining = match overall_timeout.checked_sub(elapsed) {
            Some(r) if !r.is_zero() => r,
            _ => {
                let observed: Vec<String> =
                    current.iter().map(|t| t.name.clone()).collect();
                return Err(super::Error::ToolSubscriptionTimeout {
                    expected: expected.to_vec(),
                    observed,
                });
            }
        };
        let next = observer
            .subscribe_tools(&current, remaining)
            .await
            .map_err(|e| super::Error::McpListTools(format!("{e}")))?;
        if all_present(&next, expected) {
            return Ok(());
        }
        current = next;
    }
}

fn all_present(
    returned: &[objectiveai_sdk::mcp::tool::Tool],
    expected: &[String],
) -> bool {
    expected
        .iter()
        .all(|exp| returned.iter().any(|t| matches_expected(&t.name, exp)))
}

/// Match either:
///   `objectiveai-invention_<expected>`            — single upstream
///   `objectiveai-invention_<digits>_<expected>`   — collision-disambiguated
fn matches_expected(returned_name: &str, expected: &str) -> bool {
    let Some(rest) = returned_name.strip_prefix(PROXY_INVENTION_PREFIX) else {
        return false;
    };
    if rest == expected {
        return true;
    }
    if let Some((idx, name)) = rest.split_once('_') {
        return !idx.is_empty()
            && idx.bytes().all(|b| b.is_ascii_digit())
            && name == expected;
    }
    false
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Client for inventing Functions.
///
/// Orchestrates the multi-step invention flow: essay, input schema,
/// essay tasks, tasks, description, and readme generation.
pub struct Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM> {
    pub agent_client: Arc<
        crate::agent::completions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG,
        >,
    >,
    pub github_client: Arc<crate::github::Client>,
    pub filesystem_client: Arc<crate::filesystem::Client>,
    pub retrieve_router:
        Arc<crate::retrieval::retrieve::Router<FFNG, FFNF, FFNM, CTXEXT>>,
    pub usage_handler: Arc<IUSG>,
    pub invention_server_spawner: Arc<super::InventionServerSpawner>,
    pub persist: bool,
    pub forbid_overwrite: bool,
    /// Maximum total time we'll wait — across the initial `list_tools`
    /// and all subsequent `subscribe_tools` iterations — for the
    /// agent's MCP view to show every expected tool after a between-step
    /// `set_tools` swap on the InventionServer. Exceeding this surfaces
    /// `Error::ToolSubscriptionTimeout`.
    pub subscribe_tools_timeout: time::Duration,
}

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM>
    Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM>
{
    pub fn new(
        agent_client: Arc<
            crate::agent::completions::Client<
                CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG,
            >,
        >,
        github_client: Arc<crate::github::Client>,
        filesystem_client: Arc<crate::filesystem::Client>,
        retrieve_router: Arc<
            crate::retrieval::retrieve::Router<FFNG, FFNF, FFNM, CTXEXT>,
        >,
        usage_handler: Arc<IUSG>,
        invention_server_spawner: Arc<super::InventionServerSpawner>,
        persist: bool,
        forbid_overwrite: bool,
        subscribe_tools_timeout: time::Duration,
    ) -> Self {
        Self {
            agent_client,
            github_client,
            filesystem_client,
            retrieve_router,
            usage_handler,
            invention_server_spawner,
            persist,
            forbid_overwrite,
            subscribe_tools_timeout,
        }
    }
}

type Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK> =
    crate::agent::completions::Continuation<
        <OPENROUTER as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation,
        >>::State,
        <CLAUDEAGENTSDK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        >>::State,
        <CODEXSDK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        >>::State,
        <MOCK as crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation,
        >>::State,
    >;

impl<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM>
    Client<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG, IUSG, FFNG, FFNF, FFNM>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation>
        + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    IUSG: super::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
    FFNG: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNF: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
    FFNM: crate::retrieval::retrieve::Client<CTXEXT> + Send + Sync + 'static,
{
    pub async fn create_unary_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<
            objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
        >,
    ) -> Result<
        objectiveai_sdk::functions::inventions::response::unary::FunctionInvention,
        super::Error,
    > {
        let mut aggregate: Option<FunctionInventionChunk> = None;
        let mut stream =
            self.create_streaming_handle_usage(ctx, request).await?;
        while let Some(chunk) = stream.next().await {
            match &mut aggregate {
                Some(aggregate) => aggregate.push(&chunk),
                None => aggregate = Some(chunk),
            }
        }
        Ok(aggregate.unwrap().into())
    }

    pub async fn create_streaming_handle_usage(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<
            objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
        >,
    ) -> Result<
        impl Stream<Item = FunctionInventionChunk> + Send + Unpin + 'static,
        super::Error,
    > {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut aggregate: Option<FunctionInventionChunk> = None;
            let stream = match self
                .clone()
                .create_streaming(ctx.clone(), request.clone())
                .await
            {
                Ok(stream) => stream,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            futures::pin_mut!(stream);
            while let Some(chunk) = stream.next().await {
                match &mut aggregate {
                    Some(aggregate) => aggregate.push(&chunk),
                    None => aggregate = Some(chunk.clone()),
                }
                if tx.send(Ok(chunk)).is_err() {
                    ctx.cancel();
                }
            }
            drop(stream);
            drop(tx);
            if let Some(aggregate) = aggregate {
                if aggregate.usage.as_ref().is_some_and(
                    objectiveai_sdk::agent::completions::response::Usage::any_usage,
                ) {
                    self.usage_handler
                        .handle_usage(ctx, request, aggregate.into())
                        .await;
                }
            }
        });
        let mut stream =
            tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
        match stream.next().await {
            Some(Ok(chunk)) => {
                Ok(StreamOnce::new(chunk).chain(stream.map(Result::unwrap)))
            }
            Some(Err(e)) => Err(e),
            None => unreachable!(),
        }
    }

    pub async fn create_streaming(
        self: Arc<Self>,
        ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: Arc<
            objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
        >,
    ) -> Result<
        impl Stream<Item = FunctionInventionChunk> + Send + 'static,
        super::Error,
    > {
        // Resolve state and prompt concurrently.
        let state_fut = self.retrieve_router
            .get_function_invention_state(&ctx, request.state.clone());
        let prompt_fut = self.retrieve_router
            .get_prompt(&ctx, request.prompt.clone());
        let (resolved_state, resolved_prompt) = tokio::join!(state_fut, prompt_fut);
        let resolved_state = resolved_state
            .map_err(|e| super::Error::InvalidState(e.to_string()))?
            .ok_or(super::Error::StateNotFound)?;
        let resolved_prompt = resolved_prompt
            .map_err(super::Error::PromptFetch)?;

        // Validate params before starting.
        let params = match &resolved_state {
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaScalarBranch(s) => &s.params,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaScalarLeaf(s) => &s.params,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaVectorBranch(s) => &s.params,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaVectorLeaf(s) => &s.params,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaScalar(s) => &s.params,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaVector(s) => &s.params,
        };
        params.validate().map_err(super::Error::InvalidState)?;
        validate_name(&params.name)?;

        // Validate depth matches variant.
        let is_leaf = matches!(
            &resolved_state,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaScalarLeaf(_)
            | objectiveai_sdk::functions::inventions::state::ParamsState::AlphaVectorLeaf(_)
        );
        let is_branch = matches!(
            &resolved_state,
            objectiveai_sdk::functions::inventions::state::ParamsState::AlphaScalarBranch(_)
            | objectiveai_sdk::functions::inventions::state::ParamsState::AlphaVectorBranch(_)
        );
        if is_leaf && params.depth > 0 {
            return Err(super::Error::InvalidState(
                format!("leaf state requires depth=0, got depth={}", params.depth),
            ));
        }
        if is_branch && params.depth == 0 {
            return Err(super::Error::InvalidState(
                "branch state requires depth>0, got depth=0".to_string(),
            ));
        }

        // Pre-flight: validate remote, token, and name.
        self.check_preflight(&ctx, &request, &params.name).await?;

        let created = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let id = invention_response_id(created);
        let mut state = resolved_state.route();
        state.set_checker_seed(request.seed);

        // Validate predicted tasks_length against params bounds.
        if let Some(tasks_length) = state.tasks_length() {
            let p = state.params();
            let (min, max) = match &state {
                State::AlphaScalarBranch(_) | State::AlphaVectorBranch(_) => {
                    (p.min_branch_width, p.max_branch_width)
                }
                State::AlphaScalarLeaf(_) | State::AlphaVectorLeaf(_) => {
                    (p.min_leaf_width, p.max_leaf_width)
                }
            };
            if tasks_length < min || tasks_length > max {
                return Err(super::Error::InvalidState(format!(
                    "tasks_length {} is outside bounds [{}, {}]",
                    tasks_length, min, max,
                )));
            }
        }

        // If the initial state has tasks, fetch all referenced child functions
        // and validate the initial state against them.
        let children = if let Some(full_fn) = state.build_function() {
            let children = self.retrieve_router.get_function_tasks(&ctx, objectiveai_sdk::functions::FullFunction::Remote(full_fn)).await
                .map_err(super::Error::FunctionFetch)?;
            Some(children)
        } else {
            None
        };
        state
            .validate_initial_state(children.as_ref())
            .map_err(super::Error::InvalidState)?;

        // Validate prompt supports this state type and compile step prompts.
        let prompt_type = state.prompt_type();
        if !resolved_prompt.supports_type(prompt_type) {
            return Err(super::Error::PromptUnsupportedType(
                format!("prompt does not have entries for type {:?}", prompt_type),
            ));
        }
        let p = state.params();
        let (tasks_min, tasks_max) = match &state {
            State::AlphaScalarBranch(_) | State::AlphaVectorBranch(_) => {
                (p.min_branch_width, p.max_branch_width)
            }
            State::AlphaScalarLeaf(_) | State::AlphaVectorLeaf(_) => {
                (p.min_leaf_width, p.max_leaf_width)
            }
        };
        let prompt_params = objectiveai_sdk::functions::expression::Params::Owned(
            objectiveai_sdk::functions::expression::ParamsOwned {
                input: objectiveai_sdk::functions::expression::InputValue::Object(Default::default()),
                output: None,
                map: None,
                tasks_min: Some(tasks_min),
                tasks_max: Some(tasks_max),
                depth: Some(p.depth),
                name: Some(p.name.clone()),
                spec: Some(p.spec.clone()),
            },
        );
        let compiled_prompts = CompiledPrompts {
            essay: resolved_prompt.essay_for_type(prompt_type).unwrap().clone().compile(&prompt_params).unwrap(),
            input_schema: resolved_prompt.input_schema_for_type(prompt_type).unwrap().clone().compile(&prompt_params).unwrap(),
            essay_tasks: resolved_prompt.essay_tasks_for_type(prompt_type).unwrap().clone().compile(&prompt_params).unwrap(),
            tasks: resolved_prompt.tasks_for_type(prompt_type).unwrap().clone().compile(&prompt_params).unwrap(),
            description: resolved_prompt.description_for_type(prompt_type).unwrap().clone().compile(&prompt_params).unwrap(),
            tasks_min,
        };

        let agent_client = self.agent_client.clone();
        let github_client = self.github_client.clone();
        let filesystem_client = self.filesystem_client.clone();
        let invention_server_spawner = self.invention_server_spawner.clone();
        let persist = self.persist;
        let subscribe_tools_timeout = self.subscribe_tools_timeout;

        let stream: Pin<Box<dyn Stream<Item = FunctionInventionChunk> + Send>> =
            match state {
                State::AlphaScalarBranch(s) => {
                    run_all_steps(s, agent_client, github_client, filesystem_client, invention_server_spawner, ctx, request, id, created, persist, compiled_prompts, subscribe_tools_timeout)
                }
                State::AlphaScalarLeaf(s) => {
                    run_all_steps(s, agent_client, github_client, filesystem_client, invention_server_spawner, ctx, request, id, created, persist, compiled_prompts, subscribe_tools_timeout)
                }
                State::AlphaVectorBranch(s) => {
                    run_all_steps(s, agent_client, github_client, filesystem_client, invention_server_spawner, ctx, request, id, created, persist, compiled_prompts, subscribe_tools_timeout)
                }
                State::AlphaVectorLeaf(s) => {
                    run_all_steps(s, agent_client, github_client, filesystem_client, invention_server_spawner, ctx, request, id, created, persist, compiled_prompts, subscribe_tools_timeout)
                }
            };

        Ok(stream)
    }

    /// Pre-flight checks before starting the invention flow.
    ///
    /// - If remote is GitHub: validates that `github_token` is present, valid,
    ///   and has permissions to create repos, push, and edit descriptions.
    /// - If `overwrite` is not true: checks that the name doesn't already
    ///   exist on the target remote.
    async fn check_preflight(
        &self,
        ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
        request: &objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
        name: &str,
    ) -> Result<(), super::Error> {
        let remote = match &request.remote {
            Some(r) => r,
            None => return Ok(()),
        };

        // GitHub remote: validate token and check permissions.
        if matches!(remote, objectiveai_sdk::Remote::Github) {
            let scopes = self
                .github_client
                .validate_token(ctx)
                .await?;

            // The `repo` scope grants create, push, and description edit.
            // Fine-grained tokens use different headers, so also accept
            // an empty scopes list (fine-grained tokens don't return
            // x-oauth-scopes). Classic tokens must have `repo`.
            if !scopes.is_empty() && !scopes.iter().any(|s| s == "repo" || s == "public_repo") {
                return Err(super::Error::GithubTokenMissingPermissions(
                    format!(
                        "Token must have 'repo' or 'public_repo' scope. Found: [{}]",
                        scopes.join(", "),
                    ),
                ));
            }
        }

        // Check name existence (skip if overwrite is true).
        if request.overwrite == Some(true) {
            if self.forbid_overwrite {
                return Err(super::Error::OverwriteForbidden);
            }
            return Ok(());
        }

        let exists = match remote {
            objectiveai_sdk::Remote::Github => {
                let (owner, repo) = if let Some((o, r)) = name.split_once('/') {
                    (o, r)
                } else {
                    // Cannot check without owner; skip.
                    return Ok(());
                };
                self.github_client
                    .repository_exists(ctx, owner, repo)
                    .await?
            }
            objectiveai_sdk::Remote::Filesystem => {
                let (owner, repo) = if let Some((o, r)) = name.split_once('/') {
                    (o, r)
                } else {
                    return Ok(());
                };
                self.filesystem_client.repository_exists(crate::retrieval::Kind::Functions, owner, repo)
            }
            objectiveai_sdk::Remote::Mock => crate::mock::exists(name),
        };

        if exists {
            return Err(super::Error::NameAlreadyExists(name.to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Step orchestration
// ---------------------------------------------------------------------------

/// Pre-compiled prompt strings for each invention step.
struct CompiledPrompts {
    essay: String,
    input_schema: String,
    essay_tasks: String,
    tasks: String,
    description: String,
    tasks_min: u64,
}

fn run_all_steps<T, CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG>(
    state_val: T,
    agent_client: Arc<
        crate::agent::completions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG,
        >,
    >,
    github_client: Arc<crate::github::Client>,
    filesystem_client: Arc<crate::filesystem::Client>,
    invention_server_spawner: Arc<super::InventionServerSpawner>,
    ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    request: Arc<objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams>,
    id: String,
    created: u64,
    persist: bool,
    prompts: CompiledPrompts,
    subscribe_tools_timeout: time::Duration,
) -> Pin<Box<dyn Stream<Item = FunctionInventionChunk> + Send>>
where
    T: InventionState,
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation>
        + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    Box::pin(async_stream::stream! {
        let state = Arc::new(Mutex::new(state_val));
        let params = T::params(&state);
        let object = T::object();

        // Register a tenant on the process-wide shared InventionServer.
        // The tool set for this invention is swapped between steps via
        // `set_tools`, which broadcasts a `tools/list_changed`
        // notification only to this tenant's peers.
        let invention_handle = match invention_server_spawner.get().await {
            Ok(h) => h,
            Err(e) => {
                yield FunctionInventionChunk {
                    id: id.to_string(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created,
                    object,
                    usage: None,
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: 500,
                        message: serde_json::Value::String(format!(
                            "InventionServer bootstrap failed: {e}"
                        )),
                    }),
                };
                return;
            }
        };
        let invention_session = Arc::new(
            invention_handle.register(T::essay_tools(&state)).await,
        );
        let invention_url = invention_session.url();
        // The orchestrator stamps the InventionServer's pre-minted
        // rmcp session id as `Mcp-Session-Id` on every proxy → upstream
        // request to that URL. The InventionServer's tower then finds
        // an alive session and dispatches into the per-session
        // `serve_server` task, where `InventionMcp` looks up its tool
        // router by the same id.
        let invention_server_headers: indexmap::IndexMap<String, String> =
            indexmap::indexmap! {
                "Mcp-Session-Id".to_string() => invention_session.id().to_string(),
            };

        let state_chunk = |state: &Arc<Mutex<T>>, id: &str, created, object| {
            FunctionInventionChunk {
                id: id.to_string(),
                completions: vec![],
                state: Some(state.lock().unwrap().clone().into_state()),
                path: None,
                function: None,
                created,
                object,
                usage: None,
                error: None,
            }
        };

        // Continuation carried between steps.
        let mut continuation: Option<
            Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>,
        > = None;
        // Completion index incremented across all steps.
        let mut completion_index: u64 = 0;
        // Accumulated usage across all steps.
        let mut accumulated_usage = objectiveai_sdk::agent::completions::response::Usage::default();

        // Initial state
        yield state_chunk(&state, &id, created, object);

        let mut errored = false;

        // Step 1: Essay
        let essay_validate = Arc::new({ let s = state.clone(); move || T::validate_essay(&s) });
        if essay_validate().is_err() {
        errored = false;
        // Server was already constructed with `essay_tools` above — no swap
        // needed before step 0.
        let mut step = run_step(
            agent_client.clone(), ctx.clone(), request.clone(),
            prompts.essay.clone(), invention_url.clone(),
            invention_server_headers.clone(),
            essay_validate,
            id.clone(), created, object, continuation.take(), completion_index,
            T::prompt_type(), 0, prompts.tasks_min, None,
        );
        while let Some(output) = step.next().await {
            match output {
                StepOutput::Chunk(chunk) => {
                    errored = chunk.error.is_some();
                    for c in &chunk.completions {
                        if let Some(u) = &c.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    yield chunk;
                }
                StepOutput::Continuation(c) => { continuation = Some(c); }
                StepOutput::CompletionIndex(i) => { completion_index = i; }
            }
        }
        if errored {
            yield FunctionInventionChunk {
                id: id.to_string(), completions: vec![], state: None,
                path: None, function: None, created, object,
                usage: Some(accumulated_usage), error: None,
            };
            return;
        }
        }
        yield state_chunk(&state, &id, created, object);

        // Step 2: Input Schema
        let input_schema_validate = Arc::new({ let s = state.clone(); move || T::validate_input_schema(&s) });
        if input_schema_validate().is_err() {
        errored = false;
        invention_session.set_tools(T::input_schema_tools(&state)).await;
        if let Some(observer) = continuation
            .as_ref()
            .and_then(|c| c.mcp_connection())
        {
            if let Err(e) = wait_for_tools_visible(
                observer,
                &T::input_schema_tool_names(&state),
                subscribe_tools_timeout,
            ).await {
                yield FunctionInventionChunk {
                    id: id.to_string(), completions: vec![], state: None,
                    path: None, function: None, created, object,
                    usage: Some(accumulated_usage),
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: e.status(),
                        message: e.message().unwrap_or(serde_json::Value::String(e.to_string())),
                    }),
                };
                return;
            }
        }
        let mut step = run_step(
            agent_client.clone(), ctx.clone(), request.clone(),
            prompts.input_schema.clone(), invention_url.clone(),
            invention_server_headers.clone(),
            input_schema_validate,
            id.clone(), created, object, continuation.take(), completion_index,
            T::prompt_type(), 1, prompts.tasks_min, None,
        );
        while let Some(output) = step.next().await {
            match output {
                StepOutput::Chunk(chunk) => {
                    errored = chunk.error.is_some();
                    for c in &chunk.completions {
                        if let Some(u) = &c.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    yield chunk;
                }
                StepOutput::Continuation(c) => { continuation = Some(c); }
                StepOutput::CompletionIndex(i) => { completion_index = i; }
            }
        }
        if errored {
            yield FunctionInventionChunk {
                id: id.to_string(), completions: vec![], state: None,
                path: None, function: None, created, object,
                usage: Some(accumulated_usage), error: None,
            };
            return;
        }
        }
        yield state_chunk(&state, &id, created, object);

        // Step 3: Essay Tasks
        let essay_tasks_validate = Arc::new({ let s = state.clone(); move || T::validate_essay_tasks(&s) });
        if essay_tasks_validate().is_err() {
        errored = false;
        invention_session.set_tools(T::essay_tasks_tools(&state)).await;
        if let Some(observer) = continuation
            .as_ref()
            .and_then(|c| c.mcp_connection())
        {
            if let Err(e) = wait_for_tools_visible(
                observer,
                &T::essay_tasks_tool_names(&state),
                subscribe_tools_timeout,
            ).await {
                yield FunctionInventionChunk {
                    id: id.to_string(), completions: vec![], state: None,
                    path: None, function: None, created, object,
                    usage: Some(accumulated_usage),
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: e.status(),
                        message: e.message().unwrap_or(serde_json::Value::String(e.to_string())),
                    }),
                };
                return;
            }
        }
        let mut step = run_step(
            agent_client.clone(), ctx.clone(), request.clone(),
            prompts.essay_tasks.clone(), invention_url.clone(),
            invention_server_headers.clone(),
            essay_tasks_validate,
            id.clone(), created, object, continuation.take(), completion_index,
            T::prompt_type(), 2, prompts.tasks_min, None,
        );
        while let Some(output) = step.next().await {
            match output {
                StepOutput::Chunk(chunk) => {
                    errored = chunk.error.is_some();
                    for c in &chunk.completions {
                        if let Some(u) = &c.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    yield chunk;
                }
                StepOutput::Continuation(c) => { continuation = Some(c); }
                StepOutput::CompletionIndex(i) => { completion_index = i; }
            }
        }
        if errored {
            yield FunctionInventionChunk {
                id: id.to_string(), completions: vec![], state: None,
                path: None, function: None, created, object,
                usage: Some(accumulated_usage), error: None,
            };
            return;
        }
        }
        yield state_chunk(&state, &id, created, object);

        // Step 4: Tasks (Body)
        // Pre-set predicted tasks length so validate_function can succeed
        // as soon as the correct number of tasks are appended.
        T::set_tasks_length(&state, prompts.tasks_min);
        let tasks_validate = Arc::new({ let s = state.clone(); move || T::validate_function(&s) });
        if tasks_validate().is_err() {
        errored = false;
        invention_session.set_tools(T::tasks_tools(&state)).await;
        if let Some(observer) = continuation
            .as_ref()
            .and_then(|c| c.mcp_connection())
        {
            if let Err(e) = wait_for_tools_visible(
                observer,
                &T::tasks_tool_names(&state),
                subscribe_tools_timeout,
            ).await {
                yield FunctionInventionChunk {
                    id: id.to_string(), completions: vec![], state: None,
                    path: None, function: None, created, object,
                    usage: Some(accumulated_usage),
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: e.status(),
                        message: e.message().unwrap_or(serde_json::Value::String(e.to_string())),
                    }),
                };
                return;
            }
        }
        let mut step = run_step(
            agent_client.clone(), ctx.clone(), request.clone(),
            prompts.tasks.clone(), invention_url.clone(),
            invention_server_headers.clone(),
            tasks_validate,
            id.clone(), created, object, continuation.take(), completion_index,
            T::prompt_type(), 3, prompts.tasks_min, T::input_schema_json(&state),
        );
        while let Some(output) = step.next().await {
            match output {
                StepOutput::Chunk(chunk) => {
                    errored = chunk.error.is_some();
                    for c in &chunk.completions {
                        if let Some(u) = &c.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    yield chunk;
                }
                StepOutput::Continuation(c) => { continuation = Some(c); }
                StepOutput::CompletionIndex(i) => { completion_index = i; }
            }
        }
        if errored {
            yield FunctionInventionChunk {
                id: id.to_string(), completions: vec![], state: None,
                path: None, function: None, created, object,
                usage: Some(accumulated_usage), error: None,
            };
            return;
        }
        }
        yield state_chunk(&state, &id, created, object);

        // Step 5: Description
        let description_validate = Arc::new({ let s = state.clone(); move || T::validate_description(&s) });
        if description_validate().is_err() {
        errored = false;
        invention_session.set_tools(T::description_tools(&state)).await;
        if let Some(observer) = continuation
            .as_ref()
            .and_then(|c| c.mcp_connection())
        {
            if let Err(e) = wait_for_tools_visible(
                observer,
                &T::description_tool_names(&state),
                subscribe_tools_timeout,
            ).await {
                yield FunctionInventionChunk {
                    id: id.to_string(), completions: vec![], state: None,
                    path: None, function: None, created, object,
                    usage: Some(accumulated_usage),
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: e.status(),
                        message: e.message().unwrap_or(serde_json::Value::String(e.to_string())),
                    }),
                };
                return;
            }
        }
        let mut step = run_step(
            agent_client.clone(), ctx.clone(), request.clone(),
            prompts.description.clone(), invention_url.clone(),
            invention_server_headers.clone(),
            description_validate,
            id.clone(), created, object, continuation.take(), completion_index,
            T::prompt_type(), 4, prompts.tasks_min, None,
        );
        while let Some(output) = step.next().await {
            match output {
                StepOutput::Chunk(chunk) => {
                    errored = chunk.error.is_some();
                    for c in &chunk.completions {
                        if let Some(u) = &c.inner.usage {
                            accumulated_usage.push(u);
                        }
                    }
                    yield chunk;
                }
                StepOutput::Continuation(_) => {}
                StepOutput::CompletionIndex(_) => {}
            }
        }
        if errored {
            yield FunctionInventionChunk {
                id: id.to_string(), completions: vec![], state: None,
                path: None, function: None, created, object,
                usage: Some(accumulated_usage), error: None,
            };
            return;
        }
        }
        yield state_chunk(&state, &id, created, object);

        // Step 6: Readme (programmatic)
        T::write_readme(&state);

        // Final chunk: include the built function.
        let (final_state, function) = {
            let function = T::build_function(&state);
            let s = state.lock().unwrap().clone().into_state();
            (s, function)
        };

        // Clear filesystem before publishing if not persisting.
        if !persist {
            let _ = filesystem_client.clear();
        }

        // Publish the function if remote is set and build succeeded.
        let (path, publish_error) = if function.is_some() {
            if let Some(remote) = &request.remote {
                let publish_files = final_state.serialize_into_files();
                let repo = &T::params(&state).name;
                let description = extract_description(&final_state);
                match remote {
                    objectiveai_sdk::Remote::Filesystem => {
                        match publish_filesystem(
                            &filesystem_client, &ctx, repo, &publish_files,
                        ).await {
                            Ok(path) => (Some(path), None),
                            Err(e) => (None, Some(e)),
                        }
                    }
                    objectiveai_sdk::Remote::Github => {
                        match publish_github(
                            &github_client, &filesystem_client,
                            &ctx, repo, &description, &publish_files,
                        ).await {
                            Ok(path) => (Some(path), None),
                            Err(e) => (None, Some(e)),
                        }
                    }
                    objectiveai_sdk::Remote::Mock => (None, None),
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        };

        // Clear filesystem after publishing if not persisting.
        if !persist {
            let _ = filesystem_client.clear();
        }

        yield FunctionInventionChunk {
            id: id.to_string(),
            completions: vec![],
            state: Some(final_state),
            path,
            function,
            created,
            object,
            usage: Some(accumulated_usage),
            error: publish_error.map(|e| objectiveai_sdk::error::ResponseError {
                code: e.status(),
                message: e.message().unwrap_or(serde_json::Value::Null),
            }),
        };
    })
}

// ---------------------------------------------------------------------------
// Publishing helpers
// ---------------------------------------------------------------------------

/// Extracts the description from the final invention state.
pub(crate) fn extract_description(state: &objectiveai_sdk::functions::inventions::State) -> String {
    use objectiveai_sdk::functions::inventions::State;
    match state {
        State::AlphaScalarBranch(s) => s.description.clone().unwrap_or_default(),
        State::AlphaScalarLeaf(s) => s.description.clone().unwrap_or_default(),
        State::AlphaVectorBranch(s) => s.description.clone().unwrap_or_default(),
        State::AlphaVectorLeaf(s) => s.description.clone().unwrap_or_default(),
    }
}

/// Publishes to the local filesystem. Owner is resolved internally by
/// the filesystem client from the commit-author name in `ctx`.
pub(crate) async fn publish_filesystem<CTXEXT: crate::ctx::ContextExt>(
    filesystem_client: &crate::filesystem::Client,
    ctx: &crate::ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    repo: &str,
    files: &std::collections::HashMap<&'static str, String>,
) -> Result<objectiveai_sdk::RemotePath, super::Error> {
    let file_refs: Vec<(&str, &str)> = files.iter()
        .map(|(n, c)| (*n, c.as_str()))
        .collect();

    let (owner, commit) = filesystem_client
        .publish(ctx, crate::retrieval::Kind::Functions, repo, &file_refs, &format!("publish {}", repo)).await?;

    Ok(objectiveai_sdk::RemotePath::Filesystem {
        owner,
        repository: repo.to_string(),
        commit,
    })
}

/// Publishes to GitHub. Owner is resolved internally by the GitHub client.
pub(crate) async fn publish_github<CTXEXT: ctx::ContextExt + Send + Sync>(
    github_client: &crate::github::Client,
    filesystem_client: &crate::filesystem::Client,
    ctx: &ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    repo: &str,
    description: &str,
    files: &std::collections::HashMap<&'static str, String>,
) -> Result<objectiveai_sdk::RemotePath, super::Error> {
    let file_refs: Vec<(&str, &str)> = files.iter()
        .map(|(n, c)| (*n, c.as_str()))
        .collect();
    Ok(github_client
        .publish(filesystem_client, ctx, repo, description, &file_refs)
        .await?)
}

// ---------------------------------------------------------------------------
// Single step runner — streams chunks as they arrive
// ---------------------------------------------------------------------------

/// Output from a single step.
enum StepOutput<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>
where
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation>,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation>,
    CODEXSDK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation>,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation>,
{
    Chunk(FunctionInventionChunk),
    Continuation(Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>),
    CompletionIndex(u64),
}

/// Builds `AgentCompletionCreateParams` from the invention request.
///
/// The `messages` field is only populated for the very first step (when no
/// continuation exists). For all subsequent steps and retries the prompt is
/// pushed as a user message onto the continuation so that the upstream sees
/// one continuous conversation.
fn build_agent_params(
    request: &objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams,
    messages: Vec<objectiveai_sdk::agent::completions::message::Message>,
) -> objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
    objectiveai_sdk::agent::completions::request::AgentCompletionCreateParams {
        messages,
        provider: request.provider.clone(),
        agent: request.agent.clone(),
        response_format: None,
        seed: request.seed,
        stream: Some(true),
        continuation: request.continuation.clone(),
    }
}


/// Creates a user message from a prompt string.
fn user_message(prompt: &str) -> objectiveai_sdk::agent::completions::message::UserMessage {
    objectiveai_sdk::agent::completions::message::UserMessage {
        content: objectiveai_sdk::agent::completions::message::RichContent::Text(
            prompt.to_string(),
        ),
        name: None,
    }
}

fn run_step<CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG>(
    agent_client: Arc<
        crate::agent::completions::Client<
            CTXEXT, OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK, RETRG, RETRF, RETRM, CUSG,
        >,
    >,
    ctx: ctx::Context<CTXEXT, impl crate::ctx::persistent_cache::PersistentCacheClient>,
    request: Arc<objectiveai_sdk::functions::inventions::request::FunctionInventionCreateParams>,
    prompt: String,
    invention_url: String,
    invention_server_headers: indexmap::IndexMap<String, String>,
    validate: Arc<dyn Fn() -> Result<(), String> + Send + Sync>,
    id: String,
    created: u64,
    object: Object,
    initial_continuation: Option<Continuation<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>>,
    initial_completion_index: u64,
    invention_type: objectiveai_sdk::functions::inventions::prompts::StepPromptType,
    invention_step: usize,
    invention_tasks_min: u64,
    invention_input_schema: Option<String>,
) -> Pin<
    Box<
        dyn Stream<Item = StepOutput<OPENROUTER, CLAUDEAGENTSDK, CODEXSDK, MOCK>>
            + Send,
    >,
>
where
    CTXEXT: ctx::ContextExt + Send + Sync + 'static,
    OPENROUTER: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::openrouter::Agent, objectiveai_sdk::agent::openrouter::Continuation>
        + Send
        + Sync
        + 'static,
    CLAUDEAGENTSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::claude_agent_sdk::Agent, objectiveai_sdk::agent::claude_agent_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    CODEXSDK: crate::agent::completions::UpstreamClient<
            objectiveai_sdk::agent::codex_sdk::Agent, objectiveai_sdk::agent::codex_sdk::Continuation,
        > + Send
        + Sync
        + 'static,
    MOCK: crate::agent::completions::UpstreamClient<objectiveai_sdk::agent::mock::Agent, objectiveai_sdk::agent::mock::Continuation>
        + Send
        + Sync
        + 'static,
    RETRG: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRF: crate::retrieval::retrieve::Client<CTXEXT>,
    RETRM: crate::retrieval::retrieve::Client<CTXEXT>,
    CUSG: crate::agent::completions::usage_handler::UsageHandler<CTXEXT> + Send + Sync + 'static,
{
    Box::pin(async_stream::stream! {
        let mut continuation = initial_continuation;
        let mut completion_index = initial_completion_index;
        let validate_for_done = validate.clone();
        let max_step_retries = request.max_step_retries.unwrap_or(3);

        // The InventionServer is owned by the parent `run_all_steps` and
        // shared across every step of this invention. The parent has already
        // swapped in this step's tool set and broadcast a
        // `tools/list_changed` notification before we get here, so the proxy
        // refetches `tools/list` on the next agent request and the agent
        // sees the right tools.

        // The messages array on the request is fixed after the very first
        // step. If we have a continuation (i.e. this is not the first step),
        // the prompt goes as a user message on the continuation and the
        // request messages are empty. If no continuation exists (first step),
        // the prompt goes into the request messages.
        let agent_params = Arc::new(if let Some(ref mut cont) = continuation {
            cont.push_user_message(user_message(&prompt));
            build_agent_params(&request, vec![])
        } else {
            build_agent_params(
                &request,
                vec![objectiveai_sdk::agent::completions::message::Message::User(
                    user_message(&prompt),
                )],
            )
        });

        // `disable_tools` is the orchestrator's signal that the model has
        // produced what we needed — once `validate()` succeeds, the next
        // continuation runs with tools off and the model closes out with
        // a content-only response.
        let disable_tools = Arc::new(move || validate_for_done().is_ok());

        let stream_result = agent_client
            .create_streaming(
                ctx.clone(),
                agent_params.clone(),
                continuation.take(),
                Some(disable_tools),
                vec![crate::agent::completions::ExtraMcpServer {
                    url: invention_url.clone(),
                    headers: Some(invention_server_headers.clone()),
                }],
                indexmap::IndexMap::new(),
                None,
                false,
                Some(invention_type),
                Some(invention_step),
                Some(invention_tasks_min),
                invention_input_schema.clone(),
            )
            .await;

        let stream = match stream_result {
            Ok(stream) => stream,
            Err(e) => {
                yield StepOutput::Chunk(FunctionInventionChunk {
                    id: id.clone(),
                    completions: vec![],
                    state: None,
                    path: None,
                    function: None,
                    created,
                    object,
                    usage: None,
                    error: Some(objectiveai_sdk::error::ResponseError {
                        code: {
                            use objectiveai_sdk::error::StatusError;
                            e.status()
                        },
                        message: {
                            use objectiveai_sdk::error::StatusError;
                            e.message().unwrap_or(serde_json::Value::Null)
                        },
                    }),
                });
                return;
            }
        };

        futures::pin_mut!(stream);
        while let Some(item) = stream.next().await {
            match item {
                crate::agent::completions::StreamItem::Chunk(chunk) => {
                    yield StepOutput::Chunk(FunctionInventionChunk {
                        id: id.clone(),
                        completions: vec![InventionAgentCompletionChunk {
                            index: completion_index,
                            inner: chunk,
                        }],
                        state: None,
                        path: None,
                        function: None,
                        created,
                        object,
                        usage: None,
                        error: None,
                    });
                }
                crate::agent::completions::StreamItem::State(cont) => {
                    continuation = Some(cont);
                }
            }
        }

        // If validate still fails after the agent loop ended, start a new
        // agent completion with a retry prompt on the continuation. The
        // retry prompt includes the validation error, matching the pattern
        // from objectiveai-cli: prompt + "\n\n" + error + "\n\nPlease try again."
        //
        // The loop carries the still-failing validation error out via
        // `break Some(e)` when retries exhaust, so the caller can yield
        // a terminal error chunk instead of silently advancing to the
        // next step. `break None` signals validation eventually passed.
        let mut retries = 0u32;
        let final_validation_error: Option<String> = loop {
            let validation_error = match validate() {
                Ok(()) => break None,
                Err(e) => e,
            };
            if retries >= max_step_retries {
                break Some(validation_error);
            }
            retries += 1;

            let retry_prompt = format!(
                "{}\n\nThe following error occurred: {}\n\nPlease try again.",
                prompt, validation_error,
            );

            if let Some(ref mut cont) = continuation {
                cont.push_user_message(user_message(&retry_prompt));
            }

            completion_index += 1;

            let validate_for_done = validate.clone();
            let disable_tools = Arc::new(move || validate_for_done().is_ok());

            let stream_result = agent_client
                .create_streaming(
                    ctx.clone(),
                    agent_params.clone(),
                    continuation.take(),
                    Some(disable_tools),
                    vec![crate::agent::completions::ExtraMcpServer::new(invention_url.clone())],
                    invention_server_headers.clone(),
                    None,
                    false,
                    Some(invention_type),
                    Some(invention_step),
                    Some(invention_tasks_min),
                    invention_input_schema.clone(),
                )
                .await;

            let stream = match stream_result {
                Ok(stream) => stream,
                Err(e) => {
                    yield StepOutput::Chunk(FunctionInventionChunk {
                        id: id.clone(),
                        completions: vec![],
                        state: None,
                        path: None,
                        function: None,
                        created,
                        object,
                        usage: None,
                        error: Some(objectiveai_sdk::error::ResponseError {
                            code: {
                                use objectiveai_sdk::error::StatusError;
                                e.status()
                            },
                            message: {
                                use objectiveai_sdk::error::StatusError;
                                e.message().unwrap_or(serde_json::Value::Null)
                            },
                        }),
                    });
                    return;
                }
            };

            futures::pin_mut!(stream);
            while let Some(item) = stream.next().await {
                match item {
                    crate::agent::completions::StreamItem::Chunk(chunk) => {
                        yield StepOutput::Chunk(FunctionInventionChunk {
                            id: id.clone(),
                            completions: vec![InventionAgentCompletionChunk {
                                index: completion_index,
                                inner: chunk,
                            }],
                            state: None,
                            path: None,
                            function: None,
                            created,
                            object,
                            usage: None,
                            error: None,
                        });
                    }
                    crate::agent::completions::StreamItem::State(cont) => {
                        continuation = Some(cont);
                    }
                }
            }
        };

        // Retries exhausted with validation still failing — yield a
        // terminal error chunk so `run_all_steps` aborts the invention
        // instead of advancing to the next step with an invalid state.
        // Mirrors the two `agent_client.create_streaming` Err paths
        // above: empty completions, all fields None except `error`,
        // then `return;` to skip the post-loop yields.
        if let Some(last_error) = final_validation_error {
            let attempts = retries + 1; // initial attempt plus N retries
            let err = super::Error::ValidationFailedAfterRetries {
                step: step_name(invention_step),
                attempts,
                last_error,
            };
            yield StepOutput::Chunk(FunctionInventionChunk {
                id: id.clone(),
                completions: vec![],
                state: None,
                path: None,
                function: None,
                created,
                object,
                usage: None,
                error: Some(objectiveai_sdk::error::ResponseError::from(&err)),
            });
            return;
        }

        // Yield final continuation and completion index for the next step.
        if let Some(cont) = continuation {
            yield StepOutput::Continuation(cont);
        }
        yield StepOutput::CompletionIndex(completion_index + 1);
    })
}
