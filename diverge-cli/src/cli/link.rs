//! The `link` command, done.
//!
//! # Linking the provider on this root
//!
//! 1. The key is minted into the root's keyring as `provider/<name>`.
//!    A name already held is refused, and nothing else is done.
//! 2. `provider.yaml` gains the way `{key: provider/<name>, identity}`
//!    in `auth.unbrokered`, validated and written as `provider config
//!    set` writes it.
//! 3. The daemon `cli.yaml` selects is told to dial the provider at
//!    `ws://127.0.0.1:<its port>/`, presenting the entry's key:
//!    `providers outgoing add`, the mode naming the entry.
//! 4. One JSON line says what was linked and what the daemon answered.
//!    The provider reads `provider.yaml` at its start only, so it is
//!    restarted before it accepts the key.
//!
//! A step that fails leaves the steps before it done, and says which.

use std::path::Path;

use diverge_provider::config::Config as ProviderConfig;
use diverge_provider::config::auth::{Auth, Key, Unbrokered};
use diverge_sdk::daemon::endpoints::providers::outgoing::Mode;
use diverge_sdk::daemon::endpoints::providers::outgoing::add::{client, server::response};
use diverge_sdk::keyring::Keyring;
use serde_json::json;

use super::Error;
use super::keyring::print;
use crate::arguments::Link;
use crate::config;
use crate::document::{self, Which};
use crate::run::Target;

/// Do `command` on the root.
pub async fn run(root: &Path, command: Link) -> Result<(), Error> {
    let Link::Provider {
        name,
        identity,
        daemon,
        account,
    } = command;
    let entry = format!("provider/{name}");
    let cli = config::load(root).await?;
    let selected = config::select(&cli, daemon.as_deref(), account.as_deref())?;
    let keyring = Keyring::open(root).await?;
    keyring.mint(&entry).await?;
    let port = accept(root, &entry, &identity).await?;
    let address = format!("ws://127.0.0.1:{port}/");
    let target = Target {
        address: &selected.daemon.address,
        root,
        key: &selected.account.key,
    };
    let handle = target.dial().await?;
    let request = client::request::Frame {
        address: address.clone(),
        mode: Mode::Unbrokered { key: entry.clone() },
        relay: None,
    };
    let answer = client::execute::execute(&handle, &request)
        .await
        .map_err(|error| Error::Link(format!("the daemon could not be asked: {error}")))?;
    let line = json!({
        "linked": { "entry": entry, "identity": identity, "address": address },
        "daemon": answer,
        "restart": "provider",
    });
    print(&[line]).await?;
    match answer {
        response::Frame::Added => Ok(()),
        _ => Err(Error::Link(
            "the daemon did not add the provider; the key and provider.yaml are kept".to_string(),
        )),
    }
}

/// `provider.yaml` with the way accepting `entry` as `identity`
/// appended, written; the provider's port.
async fn accept(root: &Path, entry: &str, identity: &str) -> Result<u16, Error> {
    let current = document::read(root, Which::Provider).await?;
    let mut provider: ProviderConfig =
        serde_json::from_value(current).map_err(document::Error::Json)?;
    let way = Unbrokered::Key(Key {
        key: entry.to_string(),
        identity: identity.to_string(),
        address: None,
    });
    let auth = provider.auth.get_or_insert_with(Auth::default);
    auth.unbrokered.get_or_insert_with(Vec::new).push(way);
    let value = serde_json::to_value(&provider).map_err(document::Error::Json)?;
    let yaml = Which::Provider.validate(value)?;
    document::write(root, Which::Provider, &yaml).await?;
    Ok(provider.port)
}
