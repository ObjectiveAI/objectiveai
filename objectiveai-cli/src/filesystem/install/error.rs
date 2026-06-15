use reqwest::StatusCode;
use std::path::PathBuf;

/// Failures the GitHub-install pipeline (shared by tools and plugins)
/// can encounter. Shape modelled after `objectiveai-api`'s
/// `github::Error`: split request / response / status / parse so
/// diagnostics name what failed, plus IO variants for the local-disk
/// side of the flow.
#[derive(thiserror::Error, Debug)]
pub enum InstallError {
    #[error("manifest request failed: {0}")]
    ManifestRequest(reqwest::Error),
    #[error("manifest fetch returned bad status {code} from {url}: {body}")]
    ManifestBadStatus {
        code: StatusCode,
        url: String,
        body: String,
    },
    #[error("manifest body could not be read: {0}")]
    ManifestResponse(reqwest::Error),
    #[error("manifest parse failed: {0}")]
    ManifestParse(serde_path_to_error::Error<serde_json::Error>),
    #[error("cli-zip request failed: {0}")]
    CliZipRequest(reqwest::Error),
    #[error("cli-zip fetch returned bad status {code} from {url}")]
    CliZipBadStatus { code: StatusCode, url: String },
    #[error("cli-zip body could not be read: {0}")]
    CliZipResponse(reqwest::Error),
    #[error("failed to create install directory {0}: {1}")]
    PluginDirCreate(PathBuf, std::io::Error),
    #[error("failed to serialize manifest for persistence: {0}")]
    ManifestSerialize(serde_json::Error),
    #[error("failed to persist manifest at {0}: {1}")]
    ManifestPersist(PathBuf, std::io::Error),
    #[error("viewer-zip request failed: {0}")]
    ViewerZipRequest(reqwest::Error),
    #[error("viewer-zip fetch returned bad status {code} from {url}")]
    ViewerZipBadStatus { code: StatusCode, url: String },
    #[error("viewer-zip body could not be read: {0}")]
    ViewerZipResponse(reqwest::Error),
    #[error("failed to extract zip into {0}: {1}")]
    ZipExtract(PathBuf, String),
    #[error("invalid header name {name:?}: {reason}")]
    InvalidHeaderName { name: String, reason: String },
    #[error("invalid header value for {name:?}: {reason}")]
    InvalidHeaderValue { name: String, reason: String },
    #[error(
        "{repository} is already installed; pass `--upgrade` to replace it"
    )]
    AlreadyInstalled { repository: String },
    #[error(
        "repository name {repository:?} is reserved and cannot be used as a plugin name"
    )]
    ReservedRepositoryName { repository: String },
    #[error("manifest failed validation: {0}")]
    ManifestInvalid(&'static str),
    /// `owner`, `repository`, or `commit` does not match the
    /// `^[A-Za-z0-9_.-]{1,128}$` shape required for them to land in a
    /// usable LLM tool name (Anthropic enforces `^[a-zA-Z0-9_-]{1,128}$`;
    /// we additionally allow `.` so semver-shaped version strings can
    /// flow through, with `.` -> `-` substitution applied when the tool
    /// name is materialized).
    #[error("invalid {kind}: {value:?} must match ^[A-Za-z0-9_.-]{{1,128}}$")]
    InvalidIdentifier { kind: &'static str, value: String },
}
