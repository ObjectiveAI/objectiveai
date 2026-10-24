//! The `issuer` command, done.

use std::path::Path;

use diverge_issuer::store::Store;
use diverge_issuer::{password, signing};
use diverge_sdk::file_lock;
use serde_json::json;
use tokio::io::AsyncReadExt as _;

use super::Error;
use super::print::print;
use crate::arguments::{Issuer, IssuerIdentities};
use crate::document::Which;
use crate::lifecycle::{self, Program};

/// Do `command` on the root: `start` and `stop` as the daemon's and
/// the provider's are done — see [`lifecycle`] — `config` as every
/// program's document is — see [`config`](super::config) — and the
/// identities worked on and the signing key rotated in
/// `issuer.sqlite`, printing one JSON line per answer. No line holds a password, a key,
/// or a hash of either.
pub async fn run(root: &Path, command: Issuer) -> Result<(), Error> {
    match command {
        Issuer::Lifecycle(action) => Ok(lifecycle::run(root, Program::Issuer, action).await?),
        Issuer::Config(action) => super::config::run(root, Which::Issuer, action).await,
        Issuer::Identities(command) => identities(root, command).await,
        Issuer::Rotate { sign_out } => rotate(root, sign_out).await,
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

/// The signing key rotated, printing what was done and when it holds.
/// The rules:
///
/// 1. **The issuer runs** — its root lock, `issuer.lock`, is held: the
///    issuer rotates itself, on its loopback `POST /rotate`, and signs
///    with the new key at once. Its answer is printed, with
///    `"takes_effect":"now"`. A `403` is the route refused as not from
///    this host; any other code is [`Error::Rotate`], with the
///    issuer's words.
/// 2. **The issuer does not run**: the CLI takes the root lock itself,
///    so no issuer starts half way through, rotates in
///    `issuer.sqlite`, and lets the lock go. The issuer signs with the
///    new key from its start: `"takes_effect":"at the issuer's start"`.
async fn rotate(root: &Path, sign_out: bool) -> Result<(), Error> {
    let lock = root.join(diverge_issuer::LOCK_FILE);
    let held = file_lock::try_exclusive(&lock)
        .await
        .map_err(|error| Error::Rotate(error.to_string()))?;
    let Some(held) = held else {
        let port = lifecycle::port(root, Program::Issuer).await?;
        let query = format!("/rotate?sign_out={sign_out}");
        let (code, body) = lifecycle::http::post(port, &query, "").await?;
        return match code {
            200 => {
                let mut answer: serde_json::Value =
                    serde_json::from_str(&body).map_err(|error| {
                        Error::Rotate(format!("the issuer's answer is not JSON: {error}"))
                    })?;
                if let Some(object) = answer.as_object_mut() {
                    object.insert("takes_effect".to_string(), json!("now"));
                }
                print(&[answer]).await
            }
            403 => Err(lifecycle::Error::Refused.into()),
            _ => Err(Error::Rotate(format!("the issuer answered {code}: {body}"))),
        };
    };
    let store = Store::open(root).await?;
    let rotated = signing::rotate(&store, sign_out).await;
    drop(held);
    let (rotated, _) = rotated?;
    print(&[json!({
        "kid": rotated.key.kid,
        "made": rotated.key.made,
        "removed": rotated.removed,
        "signed_out": rotated.signed_out,
        "takes_effect": "at the issuer's start",
    })])
    .await
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
