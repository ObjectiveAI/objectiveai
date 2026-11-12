//! The accounts at a root.

use std::path::{Path, PathBuf};

use super::Error;

/// The folder under the root the accounts are kept in.
pub const FOLDER: &str = "identity.diverge.network";

/// What every request names itself as: GitHub's API refuses one that
/// names nothing.
const USER_AGENT: &str = "diverge";

/// The accounts at a root, open: the folder, and the HTTP client every
/// login speaks to a provider with. Cloning it shares the client.
#[derive(Debug, Clone)]
pub struct Accounts {
    pub(super) folder: PathBuf,
    pub(super) http: reqwest::Client,
}

impl Accounts {
    /// The accounts under `<root>/identity.diverge.network/`, the
    /// folder made owner-only where it is absent. The HTTP client
    /// follows no redirect, so no provider's answer can send a request
    /// elsewhere.
    pub async fn open(root: &Path) -> Result<Accounts, Error> {
        let folder = root.join(FOLDER);
        let mut builder = tokio::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(&folder).await.map_err(|source| Error::Io { path: folder.clone(), source })?;
        let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).user_agent(USER_AGENT).build().map_err(Error::Http)?;
        Ok(Accounts { folder, http })
    }
}
