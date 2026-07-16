#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Filesystem(#[from] crate::filesystem::Error),
    #[error("{}", format_http_error(.0))]
    Http(#[from] objectiveai_sdk::HttpError),
    #[error("{0}")]
    ResponseError(objectiveai_sdk::error::ResponseError),
    #[error("{0} source is not supported for function-profile pairs")]
    PairsSourceNotSupported(&'static str),
    #[error("authorization is global only")]
    AuthorizationGlobalOnly,
    #[error("{0}")]
    MissingArgs(&'static str),
    #[error("invalid path: {0}")]
    PathParse(String),
    #[error("python wasm runtime error: {0}")]
    PythonWasm(String),
    #[error("podman: {0}")]
    Podman(String),
    #[error("laboratory: {0}")]
    Laboratory(String),
    #[error("failed to read python file {0}: {1}")]
    PythonFileRead(std::path::PathBuf, std::io::Error),
    #[error("failed to read prompt file {0}: {1}")]
    PromptFileRead(std::path::PathBuf, std::io::Error),
    #[error("failed to read JSON file {0}: {1}")]
    JsonFileRead(std::path::PathBuf, std::io::Error),
    #[error("python exception:\n{0}")]
    PythonException(String),
    #[error("python script produced no output")]
    PythonNoOutput,
    #[error("python output deserialization failed: {0}")]
    PythonDeserialize(serde_path_to_error::Error<serde_json::Error>),
    #[error("internal error: python harness output is malformed: {0}")]
    PythonHarnessBroken(String),
    #[error("inline JSON deserialization failed: {0}")]
    InlineDeserialize(serde_path_to_error::Error<serde_json::Error>),
    #[error("inline JSON conversion failed: {0}")]
    InlineJson(#[from] serde_json::Error),
    #[error("stream ended without producing any chunks")]
    EmptyStream,
    #[error(
        "the daemon cannot kill itself over /execute; run `objectiveai daemon kill` from the CLI"
    )]
    CannotKillSelf,
    #[error("config set forbidden by server configuration")]
    ConfigSetForbidden,
    #[error("log writer task panicked or was cancelled")]
    WriterPanic,
    #[error("subscribe timed out")]
    LogSubscribeTimedOut,
    #[error("plugin not found: {0}")]
    PluginNotFound(String),
    #[error("failed to spawn plugin: {0}")]
    PluginSpawn(std::io::Error),
    #[error("failed to read plugin output: {0}")]
    PluginRead(std::io::Error),
    #[error("plugin exited with non-zero status: {0}")]
    PluginExit(i32),
    #[error("plugins may not invoke `{0}` commands")]
    PluginCommandForbidden(&'static str),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("failed to spawn tool: {0}")]
    ToolSpawn(std::io::Error),
    #[error("failed to read tool output: {0}")]
    ToolRead(std::io::Error),
    #[error("tool exited with non-zero status: {0}")]
    ToolExit(i32),
    #[error(
        "{kind} {owner}/{repository} (commit {commit_sha}, version {version}) is not in the install whitelist; pass --allow-untrusted to install anyway"
    )]
    NotWhitelisted {
        kind: &'static str,
        owner: String,
        repository: String,
        commit_sha: String,
        version: String,
    },
    #[error("whitelist regex error: {0}")]
    WhitelistRegex(regex::Error),
    #[error(
        "daemon address unavailable: the daemon could not be spawned or its lock could not be read"
    )]
    DaemonAddressUnavailable,
    #[error("updater: {0}")]
    Updater(String),
    #[error("instance runner: {0}")]
    Instance(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("{0} remote is not supported for this command")]
    RemoteNotSupported(&'static str),
    #[error("{0}")]
    ClapParse(#[from] clap::Error),
    #[error("argument parse error at `{}`: {}", .0.field, .0.source)]
    FromArgs(#[from] objectiveai_sdk::cli::command::FromArgsError),
    #[error("{name} exited before publishing its lock ({status}); stdout: {stdout}; stderr: {stderr}")]
    SpawnExitedBeforePublishing {
        name: String,
        status: std::process::ExitStatus,
        stdout: String,
        stderr: String,
    },
    #[error("lockfile {key}: {source}")]
    Lockfile { key: String, source: std::io::Error },
    #[error("spawn {0}: {1}")]
    Spawn(String, std::io::Error),
    #[error(
        "no prior agent_completion_request for agent {agent_instance_hierarchy:?}; spawn the agent first with `agents spawn`"
    )]
    AgentNoPriorRequest { agent_instance_hierarchy: String },
    #[error(
        "agent {agent_instance_hierarchy:?} has no continuations available across {request_count} prior request(s); the most recent turn may still be streaming, or none have finished. Cannot fall back without a continuation."
    )]
    AgentNoContinuation {
        agent_instance_hierarchy: String,
        request_count: usize,
    },
    #[error(
        "tag {tag:?} exists but the agent has not been spawned yet (tag_group_id={tag_group_id}, parent_agent_instance_hierarchy={parent_agent_instance_hierarchy:?})"
    )]
    TagGrouped {
        tag: String,
        tag_group_id: i64,
        parent_agent_instance_hierarchy: String,
    },
    #[error("tag {0:?} is not registered")]
    TagNotFound(String),
    #[error(
        "agent instance {agent_instance_hierarchy:?} is already active (its lock is held by a live process)"
    )]
    AgentInstanceActive { agent_instance_hierarchy: String },
    #[error("agent tag {tag:?} is already being spawned (its lock is held by a live process)")]
    AgentTagActive { tag: String },
    #[error(
        "cannot apply tag {tag:?} while an agent holding it is active (its tag lock is held by a live process)"
    )]
    TagApplyAgentActive { tag: String },
    #[error(
        "cannot remove tag {tag:?} while an agent holding it is active (its tag lock is held by a live process)"
    )]
    TagRemoveAgentActive { tag: String },
    #[error("cannot enqueue against an agent ref; enqueue targets an instance or a tag")]
    EnqueueRefTarget,
    #[error("cannot wait on an agent ref; wait targets an instance or a tag")]
    WaitRefTarget,
    #[error("cannot attach/detach a laboratory to an agent ref; target an instance or a tag")]
    LaboratoryRefTarget,
    #[error("laboratory {laboratory_id:?} is already attached to this agent")]
    LaboratoryAlreadyAttached { laboratory_id: String },
    #[error("laboratory {laboratory_id:?} is not attached to this agent")]
    LaboratoryNotAttached { laboratory_id: String },
    #[error(
        "FATAL: tag {tag:?} lock was released without its GROUPED->BOUND upgrade; the spawn flow's upgrade-before-release invariant is broken"
    )]
    TagLockDroppedWithoutUpgrade { tag: String },
    #[error(
        "queued message {id} was sent by {sender_agent_instance_hierarchy:?}; it can only be deleted by the sender or a parent of the sender (caller is {caller_agent_instance_hierarchy:?})"
    )]
    QueueDeleteUnauthorized {
        id: i64,
        sender_agent_instance_hierarchy: String,
        caller_agent_instance_hierarchy: String,
    },
    #[error(
        "a schedule named {name:?} already exists for {agent_instance_hierarchy:?}; pass --overwrite to replace it"
    )]
    ScheduleAlreadyExists {
        name: String,
        agent_instance_hierarchy: String,
    },
    #[error("db: {0}")]
    Db(#[from] crate::db::Error),
    /// Endpoint exists in the command tree but the underlying
    /// implementation hasn't landed yet — typically because it depends
    /// on the postgres-backed `logs.*` reader that's still in flight.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("invalid query: {0}")]
    InvalidQuery(String),
    #[error("query exceeded timeout")]
    QueryTimeout,
    #[error("query attempted a write in a read-only context")]
    QueryReadOnlyViolation,
    /// The executor's whole-stream deadline elapsed. Yielded as the
    /// stream's final item; the stream never outlives the timeout.
    #[error("timed out after {timeout_seconds}s")]
    Timeout { timeout_seconds: u64 },
    /// The executor's running token tally over the serialized output
    /// exceeded the request's `max_tokens` budget. Yielded as the
    /// stream's final item in place of the item that pushed it over.
    #[error("response exceeded token budget — actual {actual} tokens, limit {limit}")]
    TokenBudgetExceeded { limit: u64, actual: u64 },
}

impl Error {
    /// JSON value to use when serializing this error in user-facing
    /// output. For `ResponseError` this is the inner error serialized
    /// as a structured object; for everything else it's a string built
    /// from the `Display` impl.
    pub fn output_message(&self) -> serde_json::Value {
        match self {
            Error::ResponseError(re) => {
                serde_json::to_value(re).unwrap_or_else(|_| self.to_string().into())
            }
            _ => self.to_string().into(),
        }
    }
}

fn http_is_connect_failure(err: &objectiveai_sdk::HttpError) -> bool {
    use objectiveai_sdk::HttpError as H;
    let reqwest_err = match err {
        H::StreamError(reqwest_eventsource::Error::Transport(e)) => e,
        H::RequestError(e) | H::HttpError(e) => e,
        _ => return false,
    };
    reqwest_err.is_connect() || reqwest_err.is_timeout()
}

fn format_http_error(err: &objectiveai_sdk::HttpError) -> String {
    if http_is_connect_failure(err) {
        format!(
            "{err}\n\nhint: this looks like a connection failure to the resolved API \
address. the cli auto-spawns a local objectiveai-api when no address is \
configured; if you set `api.address`, verify it \
(`objectiveai api config address get --final`) or unset it to use the \
local server (the daemon spawns and manages it automatically)"
        )
    } else {
        err.to_string()
    }
}
