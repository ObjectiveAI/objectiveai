//! A credential's key: kept in the host's keyring, judged by its hash.
//!
//! The daemon mints the key of every credential it makes — an
//! account's, an incoming provider's — and keeps it in the
//! [`keyring`](diverge_sdk::keyring) under a name the credential's
//! identity decides, and nowhere else: the record keeps the key's
//! hash, and the answer that made the credential names the entry. A
//! program on the same host presents the key by reading that entry.
//!
//! # Rules
//!
//! 1. An account's key is the entry `daemon/accounts/<identity>`; an
//!    incoming provider's is `daemon/providers/<identity>`.
//! 2. The entry is written once the record that holds its hash is
//!    committed, replacing whatever the name held: a record not
//!    committed leaves the keyring as it was.
//! 3. A credential taken out, or given another identity, takes its
//!    old entry out with it.

use diverge_sdk::keyring::{self, Key, Keyring};

/// The entry an account's key is kept in.
pub fn account(identity: &str) -> String {
    format!("daemon/accounts/{identity}")
}

/// The entry an incoming provider's key is kept in.
pub fn provider(identity: &str) -> String {
    format!("daemon/providers/{identity}")
}

/// The SHA-256 of a presented credential, as hex: what the store
/// keeps, and what a presented credential is looked up by. Equal keys
/// hash equal, and a hash tells nobody the key, so a copy of the
/// records is not a copy of every credential.
pub fn hash(credential: &str) -> String {
    Key::given(credential.to_string()).hash()
}

/// Keep `key` as `entry`, once its record is committed.
pub async fn keep(keyring: &Keyring, entry: &str, key: &Key) -> Result<(), keyring::Error> {
    keyring.replace(entry, key.clone()).await
}

/// Take the entry `entry` out, once the record that named it is gone
/// or names another.
pub async fn forget(keyring: &Keyring, entry: &str) -> Result<(), keyring::Error> {
    keyring.remove(entry).await.map(|_| ())
}
