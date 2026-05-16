use reqwest::StatusCode;
use std::path::PathBuf;

/// Failures the plugin install pipeline can encounter. Shape modelled
/// after `objectiveai-api`'s `github::Error`: split request / response
/// / status / parse so diagnostics name what failed, plus IO variants
/// for the local-disk side of the flow.
///
/// **Platform-not-supported is NOT an error.** When the current
/// platform isn't in `Manifest.binaries`, `Client::install_plugin`
/// returns `Ok(false)` rather than any variant here. This enum is only
/// for things that actually went wrong.
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
    #[error("binary request failed: {0}")]
    BinaryRequest(reqwest::Error),
    #[error("binary fetch returned bad status {code} from {url}")]
    BinaryBadStatus { code: StatusCode, url: String },
    #[error("binary body could not be read: {0}")]
    BinaryResponse(reqwest::Error),
    #[error("failed to create plugin directory {0}: {1}")]
    PluginDirCreate(PathBuf, std::io::Error),
    #[error("failed to write plugin binary {0}: {1}")]
    BinaryWrite(PathBuf, std::io::Error),
    #[error("failed to set executable permission on {0}: {1}")]
    Chmod(PathBuf, std::io::Error),
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
    #[error("failed to extract viewer zip into {0}: {1}")]
    ViewerZipExtract(PathBuf, String),
    #[error("invalid header name {name:?}: {reason}")]
    InvalidHeaderName { name: String, reason: String },
    #[error("invalid header value for {name:?}: {reason}")]
    InvalidHeaderValue { name: String, reason: String },
    #[error("plugin {repository} is already installed; pass `--upgrade` to replace it")]
    AlreadyInstalled { repository: String },
    #[error("repository name {repository:?} is reserved and cannot be used as a plugin name")]
    ReservedRepositoryName { repository: String },
}
