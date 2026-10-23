//! The keys, by entry.

use std::collections::HashMap;

use diverge_sdk::keyring::{Key, Keyring};

use super::Error;
use crate::config::Config;
use crate::config::auth::Unbrokered;

/// Every key the document names, by the entry it names. Nothing else
/// is in it.
#[derive(Debug, Default)]
pub struct Keys(HashMap<String, Key>);

impl Keys {
    /// Read every entry `config` names from `keyring`.
    pub async fn resolve(config: &Config, keyring: &Keyring) -> Result<Keys, Error> {
        let mut keys = HashMap::new();
        for entry in entries(config) {
            if keys.contains_key(entry) {
                continue;
            }
            let key = keyring.get(entry).await.map_err(Error::Keyring)?.ok_or_else(|| Error::Missing(entry.to_string()))?;
            keys.insert(entry.to_string(), key);
        }
        Ok(Keys(keys))
    }

    /// The key of `entry`, which [`resolve`](Self::resolve) read when
    /// the document named it; `None` for one it did not.
    pub fn get(&self, entry: &str) -> Option<&Key> {
        self.0.get(entry)
    }
}

/// Every entry the document names: the keys `auth.unbrokered`
/// accepts, the keys `clients.unbrokered` presents, and the
/// registries' passwords.
fn entries(config: &Config) -> Vec<&str> {
    let accepted = config.auth.iter().flat_map(|auth| auth.unbrokered.iter().flatten()).filter_map(|way| match way {
        Unbrokered::Key(key) => Some(key.key.as_str()),
        Unbrokered::Hook(_) => None,
    });
    let presented = config.clients.iter().flat_map(|clients| clients.unbrokered.iter().flatten()).map(|peer| peer.key.as_str());
    let passwords = config.containers.podman.registries.iter().filter_map(|registry| registry.credential.as_ref()).map(|credential| credential.password.as_str());
    accepted.chain(presented).chain(passwords).collect()
}
