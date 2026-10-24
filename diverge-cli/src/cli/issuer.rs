//! The `issuer` command, done.

use std::path::Path;

use diverge_issuer::password;
use diverge_issuer::store::Store;
use diverge_issuer::store::clients::{self, Kind, New};
use diverge_sdk::keyring::{Key, Keyring};
use serde_json::json;

use super::Error;
use super::keyring::{print, read_key};
use crate::arguments::{Clients, Grant, Identities, Issuer, Lifecycle};
use crate::lifecycle::{self, Program};

/// Do `command` on the root: `start` and `stop` as the daemon's and
/// the provider's are done — see [`lifecycle`] — and the identities
/// and the clients worked on in `issuer.sqlite`, printing one JSON
/// line per answer. No line holds a password, a secret, or a hash of
/// either.
pub async fn run(root: &Path, command: Issuer) -> Result<(), Error> {
    match command {
        Issuer::Start => Ok(lifecycle::run(root, Program::Issuer, Lifecycle::Start).await?),
        Issuer::Stop => Ok(lifecycle::run(root, Program::Issuer, Lifecycle::Stop).await?),
        Issuer::Identities(command) => identities(root, command).await,
        Issuer::Clients(command) => clients(root, command).await,
    }
}

/// An identities command, done.
async fn identities(root: &Path, command: Identities) -> Result<(), Error> {
    let store = Store::open(root).await?;
    let identities = store.identities();
    let lines = match command {
        Identities::Add { subject } => {
            let hash = password::hash(read_key().await?.expose()).await?;
            identities.add(&subject, &hash).await?;
            vec![json!({ "added": subject })]
        }
        Identities::List => identities
            .list()
            .await?
            .into_iter()
            .map(|identity| json!({ "subject": identity.subject, "made": identity.made }))
            .collect(),
        Identities::Remove { subject } => {
            vec![json!({ "removed": identities.remove(&subject).await? })]
        }
        Identities::Password { subject } => {
            let hash = password::hash(read_key().await?.expose()).await?;
            vec![json!({ "changed": identities.password(&subject, &hash).await? })]
        }
    };
    print(&lines).await
}

/// A clients command, done.
async fn clients(root: &Path, command: Clients) -> Result<(), Error> {
    let store = Store::open(root).await?;
    let lines = match command {
        Clients::Add {
            id,
            public,
            confidential: _,
            grants,
            audiences,
            redirects,
        } => vec![add(root, &store, id, public, grants, audiences, redirects).await?],
        Clients::List => store
            .clients()
            .list()
            .await?
            .into_iter()
            .map(|client| {
                let grants: Vec<&str> = client.grants.iter().map(|grant| grant.name()).collect();
                json!({
                    "id": client.id,
                    "kind": client.kind.name(),
                    "grants": grants,
                    "audiences": client.audiences,
                    "redirects": client.redirects,
                    "made": client.made,
                })
            })
            .collect(),
        Clients::Remove { id } => {
            let removed = store.clients().remove(&id).await?;
            let keyring = Keyring::open(root).await?;
            let secret = keyring.remove(&entry(&id)).await?;
            vec![json!({ "removed": removed, "secret_removed": secret })]
        }
    };
    print(&lines).await
}

/// Add the client: for a confidential one, its secret minted into the
/// keyring first, and taken back out if the client could not be
/// added, so that a secret is kept exactly when its client is.
async fn add(
    root: &Path,
    store: &Store,
    id: String,
    public: bool,
    grants: Vec<Grant>,
    audiences: Vec<String>,
    redirects: Vec<String>,
) -> Result<serde_json::Value, Error> {
    let kind = if public {
        Kind::Public
    } else {
        Kind::Confidential
    };
    let mut client = New {
        id,
        kind,
        secret_hash: None,
        grants: grants.into_iter().map(grant).collect(),
        audiences,
        redirects,
    };
    client.check()?;
    if kind == Kind::Public {
        store.clients().add(&client).await?;
        return Ok(json!({ "added": client.id, "kind": kind.name() }));
    }
    let keyring = Keyring::open(root).await?;
    let entry = entry(&client.id);
    let secret: Key = keyring.mint(&entry).await?;
    client.secret_hash = Some(secret.hash());
    if let Err(error) = store.clients().add(&client).await {
        keyring.remove(&entry).await?;
        return Err(error.into());
    }
    Ok(json!({ "added": client.id, "kind": kind.name(), "secret": entry }))
}

/// The keyring entry a confidential client's secret is kept in.
fn entry(id: &str) -> String {
    format!("issuer/clients/{id}")
}

/// The issuer's grant the CLI's names.
fn grant(grant: Grant) -> clients::Grant {
    match grant {
        Grant::AuthorizationCode => clients::Grant::AuthorizationCode,
        Grant::DeviceCode => clients::Grant::DeviceCode,
        Grant::RefreshToken => clients::Grant::RefreshToken,
        Grant::ClientCredentials => clients::Grant::ClientCredentials,
    }
}
