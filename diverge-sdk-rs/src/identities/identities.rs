//! The identities at a root.

use std::path::{Path, PathBuf};

use super::Error;

/// The folder under the root the identities are kept in.
pub const FOLDER: &str = "identities";

/// The identities at a root, open: the folder, and the HTTP client
/// every login and refresh speaks to an issuer with. Cloning it shares
/// the client.
#[derive(Debug, Clone)]
pub struct Identities {
    pub(super) folder: PathBuf,
    pub(super) http: reqwest::Client,
}

impl Identities {
    /// The identities under `<root>/identities/`, the folder made
    /// owner-only where it is absent. The HTTP client follows no
    /// redirect, so no issuer's answer can send a request elsewhere.
    pub async fn open(root: &Path) -> Result<Identities, Error> {
        let folder = root.join(FOLDER);
        folder_owned(&folder).await?;
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(Error::Http)?;
        Ok(Identities { folder, http })
    }
}

/// Make `folder`, and those above it, where absent: owner-only on
/// Unix.
pub(super) async fn folder_owned(folder: &Path) -> Result<(), Error> {
    let mut builder = tokio::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(folder).await.map_err(|source| Error::Io { path: folder.to_path_buf(), source })
}
