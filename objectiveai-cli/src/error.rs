#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Filesystem(#[from] objectiveai_sdk::filesystem::Error),
    #[error("viewer subprocess failed to start: {0}")]
    ViewerSpawn(std::io::Error),
    #[error("viewer subprocess did not report its bound address")]
    ViewerProtocol,
    #[error("api setup failed: {0}")]
    ApiSetup(std::io::Error),
    #[error("viewer config has secret but no signature, or signature but no secret")]
    ViewerSecretSignatureConfigMismatch,
    #[error("VIEWER_SECRET env var set without VIEWER_SIGNATURE, or vice versa")]
    ViewerSecretSignatureEnvMismatch,
    #[error("{0}")]
    Http(#[from] objectiveai_sdk::HttpError),
    #[error("{0}")]
    ResponseError(objectiveai_sdk::error::ResponseError),
    #[error("{0} source is not supported for function-profile pairs")]
    PairsSourceNotSupported(&'static str),
    #[error("favorite not found: {0}")]
    FavoriteNotFound(String),
    #[error("{0}")]
    MissingArgs(&'static str),
    #[error("no python interpreter found (install Python or enable the rustpython feature)")]
    PythonNotFound,
    #[error("failed to read python file {0}: {1}")]
    PythonFileRead(std::path::PathBuf, std::io::Error),
    #[error("python exception:\n{0}")]
    PythonException(String),
    #[error("python output deserialization failed: {0}")]
    PythonDeserialize(serde_path_to_error::Error<serde_json::Error>),
    #[error("internal error: python harness output is malformed: {0}")]
    PythonHarnessBroken(String),
    #[error("inline JSON deserialization failed: {0}")]
    InlineDeserialize(serde_path_to_error::Error<serde_json::Error>),
    #[error("stream ended without producing any chunks")]
    EmptyStream,
    #[error("config set forbidden by server configuration")]
    ConfigSetForbidden,
    #[error("log writer task panicked or was cancelled")]
    WriterPanic,
    #[error("unknown --instructions-id: run the matching `instructions` subcommand to get one")]
    UnknownInstructionsId,
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
    #[error("plugin {owner}/{repository} (commit {commit_sha}, version {version}) is not in the install whitelist; pass --allow-untrusted to install anyway")]
    PluginNotWhitelisted {
        owner: String,
        repository: String,
        commit_sha: String,
        version: String,
    },
    #[error("whitelist regex error: {0}")]
    WhitelistRegex(regex::Error),
}

impl Error {
    pub fn to_output(
        &self,
        level: objectiveai_sdk::cli::output::Level,
        fatal: bool,
    ) -> objectiveai_sdk::cli::output::Error {
        objectiveai_sdk::cli::output::Error {
            level,
            fatal,
            message: self.output_message(),
        }
    }

    /// JSON value to use for the `message` field of `Output::Error`.
    /// For `ResponseError` this is the inner error serialized as a
    /// structured object; for everything else it's a string built from
    /// the `Display` impl.
    pub fn output_message(&self) -> serde_json::Value {
        match self {
            Error::ResponseError(re) => {
                serde_json::to_value(re).unwrap_or_else(|_| self.to_string().into())
            }
            _ => self.to_string().into(),
        }
    }
}
