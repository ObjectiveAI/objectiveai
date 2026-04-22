#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Filesystem(#[from] objectiveai::filesystem::Error),
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
    Http(#[from] objectiveai::HttpError),
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
}
