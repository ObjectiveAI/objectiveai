//! The `issuer` command, done.

use std::path::Path;

use diverge_issuer::store::Store;
use diverge_issuer::signing;
use diverge_sdk::file_lock;
use serde_json::json;

use super::Error;
use super::print::print;
use crate::arguments::{Issuer, IssuerIdentities};
use crate::document::Which;
use crate::lifecycle::{self, Program};

/// Do `command` on the root: `start` and `stop` as the daemon's and
/// the provider's are done — see [`lifecycle`] — `config` as every
/// program's document is — see [`config`](super::config) — and the
/// identities worked on and the signing key rotated in
/// `issuer.sqlite`, and the device codes waiting decided by the running
/// issuer, printing one JSON line per answer. No line holds a key, a
/// token, or a hash of either.
pub async fn run(root: &Path, command: Issuer) -> Result<(), Error> {
    match command {
        Issuer::Lifecycle(action) => Ok(lifecycle::run(root, Program::Issuer, action).await?),
        Issuer::Config(action) => super::config::run(root, Which::Issuer, action).await,
        Issuer::Identities(command) => identities(root, command).await,
        Issuer::Rotate { sign_out } => rotate(root, sign_out).await,
        Issuer::Pending => operator(root, "/pending", "").await,
        Issuer::Approve { code } => operator(root, "/approve", &decision(&code)).await,
        Issuer::Deny { code } => operator(root, "/deny", &decision(&code)).await,
    }
}

/// An identities command, done.
async fn identities(root: &Path, command: IssuerIdentities) -> Result<(), Error> {
    let store = Store::open(root).await?;
    let identities = store.identities();
    let lines = match command {
        IssuerIdentities::List => identities
            .list()
            .await?
            .into_iter()
            .map(|identity| json!({ "subject": identity.subject, "made": identity.made }))
            .collect(),
        IssuerIdentities::Remove { subject } => {
            vec![json!({ "removed": identities.remove(&subject).await? })]
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

/// The operator's word to the running issuer, on its loopback `path`,
/// printing its answer. The rules:
///
/// 1. **The issuer runs**, or nothing is done: a root lock,
///    `issuer.lock`, that is free is [`Error::NotRunning`], since a
///    device code only waits while the issuer runs to answer its polls.
/// 2. **Its answer printed.** A `200` is the issuer's JSON: for
///    `/pending` an array, printed one line per code; else one line.
/// 3. **Refused.** A `403` is the route refused as not from this host;
///    a `404` is no pending code being that one; any other code is
///    [`Error::Operator`], with the issuer's words.
async fn operator(root: &Path, path: &str, form: &str) -> Result<(), Error> {
    let lock = root.join(diverge_issuer::LOCK_FILE);
    let free = file_lock::try_exclusive(&lock)
        .await
        .map_err(|error| Error::Operator(error.to_string()))?;
    // Rule 1.
    if free.is_some() {
        return Err(Error::NotRunning);
    }
    let port = lifecycle::port(root, Program::Issuer).await?;
    let (code, body) = lifecycle::http::post(port, path, form).await?;
    let answer = || -> Result<serde_json::Value, Error> {
        serde_json::from_str(&body).map_err(|error| Error::Operator(format!("the issuer's answer is not JSON: {error}")))
    };
    match code {
        // Rule 2.
        200 => match answer()? {
            serde_json::Value::Array(lines) => print(&lines).await,
            line => print(&[line]).await,
        },
        // Rule 3.
        403 => Err(lifecycle::Error::Refused.into()),
        _ => Err(Error::Operator(format!("the issuer answered {code}: {body}"))),
    }
}

/// The form `/approve` and `/deny` take: the user code, every byte but
/// an ASCII letter, digit or `-` percent-encoded.
fn decision(code: &str) -> String {
    let mut form = String::from("user_code=");
    for byte in code.bytes() {
        if byte.is_ascii_alphanumeric() || byte == b'-' {
            form.push(char::from(byte));
        } else {
            form.push_str(&format!("%{byte:02X}"));
        }
    }
    form
}
