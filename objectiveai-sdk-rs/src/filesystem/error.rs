#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to read directory {0}: {1}")]
    ReadDir(std::path::PathBuf, std::io::Error),
    #[error("failed to read file {0}: {1}")]
    Read(std::path::PathBuf, std::io::Error),
    #[error("failed to parse file {0}: {1}")]
    Parse(std::path::PathBuf, serde_json::Error),
    #[error("failed to serialize: {0}")]
    Serialize(serde_json::Error),
    #[error("failed to write file {0}: {1}")]
    Write(std::path::PathBuf, std::io::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid path: {0}")]
    InvalidPath(String),
    #[error("index {0} out of bounds (len {1})")]
    IndexOutOfBounds(usize, usize),
    #[error("favorite not found: {0}")]
    FavoriteNotFound(String),
    #[error("invalid favorite name (alphanumeric and dashes only, max 64 chars): {0}")]
    InvalidFavoriteName(String),
    #[error("invalid favorite note (max 512 chars): {0}")]
    InvalidFavoriteNote(String),
    #[error("remote {0:?} is not valid for configuration")]
    InvalidRemote(crate::Remote),
    #[error("jq parse error: {0}")]
    JqParse(String),
    #[error("jq compile error: {0}")]
    JqCompile(String),
    #[error("jq runtime error: {0}")]
    JqRuntime(String),
    #[error("invalid repository name (alphanumeric and dashes only, max 100 chars): {0}")]
    InvalidRepositoryName(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("db error: {0}")]
    Db(#[from] rusqlite::Error),
    #[cfg(feature = "http")]
    #[error("plugin install error: {0}")]
    Install(#[from] crate::filesystem::plugins::InstallError),
}
