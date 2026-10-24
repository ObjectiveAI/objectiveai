//! The `issuer` command, done.

use std::path::Path;

use diverge_issuer::store::Store;
use diverge_issuer::{password, signing};
use serde_json::json;
use tokio::io::AsyncReadExt as _;

use super::Error;
use super::print::print;
use crate::arguments::{Issuer, IssuerIdentities, IssuerKeys};
use crate::document::Which;
use crate::lifecycle::{self, Program};

/// Do `command` on the root: `start` and `stop` as the daemon's and
/// the provider's are done — see [`lifecycle`] — `config` as every
/// program's document is — see [`config`](super::config) — and the
/// identities and the signing keys worked on in `issuer.sqlite`,
/// printing one JSON line per answer. No line holds a password, a key,
/// or a hash of either.
pub async fn run(root: &Path, command: Issuer) -> Result<(), Error> {
    match command {
        Issuer::Lifecycle(action) => Ok(lifecycle::run(root, Program::Issuer, action).await?),
        Issuer::Config(action) => super::config::run(root, Which::Issuer, action).await,
        Issuer::Identities(command) => identities(root, command).await,
        Issuer::Keys(command) => keys(root, command).await,
    }
}

/// An identities command, done.
async fn identities(root: &Path, command: IssuerIdentities) -> Result<(), Error> {
    let store = Store::open(root).await?;
    let identities = store.identities();
    let lines = match command {
        IssuerIdentities::Add { subject } => {
            let hash = password::hash(&read_password().await?).await?;
            identities.add(&subject, &hash).await?;
            vec![json!({ "added": subject })]
        }
        IssuerIdentities::List => identities
            .list()
            .await?
            .into_iter()
            .map(|identity| json!({ "subject": identity.subject, "made": identity.made }))
            .collect(),
        IssuerIdentities::Remove { subject } => {
            vec![json!({ "removed": identities.remove(&subject).await? })]
        }
        IssuerIdentities::Password { subject } => {
            let hash = password::hash(&read_password().await?).await?;
            vec![json!({ "changed": identities.password(&subject, &hash).await? })]
        }
    };
    print(&lines).await
}

/// A keys command, done. A rotation's line says it holds from the
/// issuer's next start.
async fn keys(root: &Path, command: IssuerKeys) -> Result<(), Error> {
    let store = Store::open(root).await?;
    let lines = match command {
        IssuerKeys::Rotate { sign_out } => {
            let rotated = signing::rotate(&store, sign_out).await?;
            vec![json!({
                "kid": rotated.key.kid,
                "made": rotated.key.made,
                "retired": rotated.retired,
                "signed_out": rotated.signed_out,
                "takes_effect": "at the issuer's next start",
            })]
        }
        IssuerKeys::List => store
            .keys()
            .list()
            .await?
            .into_iter()
            .map(|key| json!({ "kid": key.kid, "made": key.made, "retired": key.retired }))
            .collect(),
    };
    print(&lines).await
}

/// The password on stdin, all of it, the trailing line end dropped.
/// An empty one is refused.
async fn read_password() -> Result<String, Error> {
    let mut text = String::new();
    tokio::io::stdin()
        .read_to_string(&mut text)
        .await
        .map_err(Error::Stdin)?;
    let trimmed = text.trim_end_matches(['\r', '\n']).len();
    text.truncate(trimmed);
    if text.is_empty() {
        return Err(Error::EmptyPassword);
    }
    Ok(text)
}
