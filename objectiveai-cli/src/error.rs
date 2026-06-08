#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Filesystem(#[from] crate::filesystem::Error),
    /// The spawn / message call failed AND the follow-up
    /// `re_enqueue_async` (which tries to restore the drained queue
    /// rows so they can be retried) also failed. Both errors are
    /// surfaced so the caller can see what went wrong with delivery
    /// *and* see that the queued content is gone.
    #[error(
        "queued content lost: the call failed AND restoring the drained queue items also failed.\n  call error: {original}\n  re-enqueue error: {re_enqueue}"
    )]
    DrainLost {
        original: Box<Error>,
        re_enqueue: Box<Error>,
    },
    #[error("{}", format_http_error(.0))]
    Http(#[from] objectiveai_sdk::HttpError),
    #[error("{0}")]
    ResponseError(objectiveai_sdk::error::ResponseError),
    #[error("{0} source is not supported for function-profile pairs")]
    PairsSourceNotSupported(&'static str),
    #[error("favorite not found: {0}")]
    FavoriteNotFound(String),
    #[error("{0}")]
    MissingArgs(&'static str),
    #[error("invalid path: {0}")]
    PathParse(String),
    #[error("no python interpreter found (install Python or enable the rustpython feature)")]
    PythonNotFound,
    #[error("failed to read python file {0}: {1}")]
    PythonFileRead(std::path::PathBuf, std::io::Error),
    #[error("failed to read prompt file {0}: {1}")]
    PromptFileRead(std::path::PathBuf, std::io::Error),
    #[error("failed to read JSON file {0}: {1}")]
    JsonFileRead(std::path::PathBuf, std::io::Error),
    #[error("python exception:\n{0}")]
    PythonException(String),
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
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("failed to spawn tool: {0}")]
    ToolSpawn(std::io::Error),
    #[error("failed to read tool output: {0}")]
    ToolRead(std::io::Error),
    #[error("tool exited with non-zero status: {0}")]
    ToolExit(i32),
    #[error(
        "plugin {owner}/{repository} (commit {commit_sha}, version {version}) is not in the install whitelist; pass --allow-untrusted to install anyway"
    )]
    PluginNotWhitelisted {
        owner: String,
        repository: String,
        commit_sha: String,
        version: String,
    },
    #[error("whitelist regex error: {0}")]
    WhitelistRegex(regex::Error),
    #[error(
        "viewer address is not configured; set VIEWER_ADDRESS in the env or run `objectiveai viewer address config set <addr>` (and optionally `objectiveai viewer port config set <port>`)"
    )]
    ViewerAddressNotConfigured,
    #[error("viewer path must start with `/`, got {0:?}")]
    ViewerPathMissingSlash(String),
    #[error("viewer body is not valid JSON: {0}")]
    ViewerBodyJsonParse(String),
    #[error("viewer http error: {0}")]
    ViewerSendHttp(String),
    #[error("viewer returned status {status}: {body}")]
    ViewerSendBadStatus { status: u16, body: String },
    #[error("updater: {0}")]
    Updater(String),
    #[error("instance runner: {0}")]
    Instance(String),
    #[error("invalid agent definition: {0}")]
    AgentConvert(String),
    #[error("{0}")]
    ClapParse(#[from] clap::Error),
    #[error("argument parse error at `{}`: {}", .0.field, .0.source)]
    FromArgs(#[from] objectiveai_sdk::cli::command::FromArgsError),
    #[error("{name} is already running (pids: {pids:?})")]
    AlreadyRunning { name: String, pids: Vec<u32> },
    #[error("{name} did not announce \"listening\" on stderr before exiting")]
    SpawnNoListeningLine { name: String },
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
        "tag {tag:?} exists but the agent has not been spawned yet (waiting on agent_full_id={agent_full_id:?} under parent_agent_instance_hierarchy={parent_agent_instance_hierarchy:?})"
    )]
    TagPending {
        tag: String,
        parent_agent_instance_hierarchy: String,
        agent_full_id: String,
    },
    #[error("tag {0:?} is not registered")]
    TagNotFound(String),
    #[error("embedded postgres bootstrap failed: {0}")]
    PostgresBootstrap(String),
    #[error("db: {0}")]
    Db(#[from] crate::db::Error),
    /// Endpoint exists in the command tree but the underlying
    /// implementation hasn't landed yet — typically because it depends
    /// on the postgres-backed `logs.*` reader that's still in flight.
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
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
            "{err}\n\nhint: this looks like a connection failure to the configured API address. \
to run an API locally:\n  \
  1. configure address + port (use an available port):\n     \
       objectiveai api address config set 127.0.0.1\n     \
       objectiveai api port config set <port>\n  \
  2. spawn the server:\n     \
       objectiveai api spawn"
        )
    } else {
        err.to_string()
    }
}
