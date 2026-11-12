//! The accounts at a root.

use std::path::{Path, PathBuf};

use super::Error;

/// The folder under the root the accounts are kept in.
pub const FOLDER: &str = "identity.diverge.network";

/// The issuer identifier of `identity.diverge.network`: the `iss` of
/// every token it signs, exactly, and so what an identity signed in
/// there has as its issuer. An issuer that is any other string — this
/// one with a trailing slash, another deployment's — is not it.
pub const ISSUER: &str = "https://identity.diverge.network";

/// Where `identity.diverge.network` is: at its issuer identifier.
pub const SERVICE: &str = ISSUER;

/// The environment variable that says where `identity.diverge.network`
/// is asked instead: another deployment of it, or one at a desk.
pub const SERVICE_VARIABLE: &str = "DIVERGE_IDENTITY_URL";

/// What every request names itself as: GitHub's API refuses one that
/// names nothing.
const USER_AGENT: &str = "diverge";

/// The accounts at a root, open: the folder, where
/// `identity.diverge.network` is, and the HTTP client every request
/// is made with. Cloning it shares the client.
#[derive(Debug, Clone)]
pub struct Accounts {
    pub(super) folder: PathBuf,
    pub(super) service: String,
    pub(super) http: reqwest::Client,
}

impl Accounts {
    /// The accounts under `<root>/identity.diverge.network/`, the
    /// folder made owner-only where it is absent. The rules:
    ///
    /// 1. `identity.diverge.network` is asked at [`SERVICE`], or at
    ///    what [`SERVICE_VARIABLE`] holds where it is set and not
    ///    empty, less any trailing slash.
    /// 2. The HTTP client follows no redirect, so no answer can send a
    ///    request, and the token it carries, elsewhere.
    pub async fn open(root: &Path) -> Result<Accounts, Error> {
        let folder = root.join(FOLDER);
        let mut builder = tokio::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder.create(&folder).await.map_err(|source| Error::Io { path: folder.clone(), source })?;
        let service = std::env::var(SERVICE_VARIABLE).ok().filter(|address| !address.is_empty()).unwrap_or_else(|| SERVICE.to_string());
        let http = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).user_agent(USER_AGENT).build().map_err(Error::Http)?;
        Ok(Accounts { folder, service: service.trim_end_matches('/').to_string(), http })
    }
}
