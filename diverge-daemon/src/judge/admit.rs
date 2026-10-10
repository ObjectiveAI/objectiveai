//! The handshake: a credential and an address become an account, a
//! provider, or nothing.

use std::net::IpAddr;

use super::{Who, key};
use crate::store::{self, Store, accounts, providers_incoming};

/// Who a credential admitted: a client, served as an account, or a
/// provider that dialled in, known by the identity its credential
/// names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Peer {
    /// A client, as the account.
    Client(Who),
    /// A provider, as the identity its credential names, with the
    /// hash of the credential, which the connection holds.
    Provider {
        /// The identity.
        identity: String,
        /// The credential's hash.
        key_hash: String,
    },
}

/// The account `credential` names, presented from `address`: the one
/// whose key hashes to the credential's hash, provided its credential
/// names no address or names this one. `None` is a refusal, and the
/// caller closes the socket without a word, as the wire requires; a
/// store that cannot be read is an error, and the caller closes the
/// socket the same way, since nothing can be said to a peer that has
/// not been admitted.
///
/// Addresses are compared canonical, so a peer the OS reports as an
/// IPv4-mapped IPv6 address matches a credential given the IPv4.
pub async fn admit(store: &Store, credential: &str, address: IpAddr) -> Result<Option<Who>, store::Error> {
    let mut conn = store.acquire().await?;
    let Some(account) = accounts::by_key_hash(&mut conn, &key::hash(credential)).await? else {
        return Ok(None);
    };
    if !accepts(account.address, address) {
        return Ok(None);
    }
    Ok(Some(Who::Account(account.id)))
}

/// The provider `credential` names, presented from `address`: the
/// identity of the incoming credential whose key hashes to it, by the
/// same rule as an account's, with that hash.
pub async fn admit_provider(store: &Store, credential: &str, address: IpAddr) -> Result<Option<Peer>, store::Error> {
    let mut conn = store.acquire().await?;
    let key_hash = key::hash(credential);
    let Some(incoming) = providers_incoming::by_key_hash(&mut conn, &key_hash).await? else {
        return Ok(None);
    };
    if !accepts(incoming.address, address) {
        return Ok(None);
    }
    Ok(Some(Peer::Provider {
        identity: incoming.identity,
        key_hash,
    }))
}

/// Who `credential` admits: a client and a provider dial the one
/// port and present the one kind of frame, so both are asked, the
/// accounts first. A key is minted at random for either, so one that
/// both hold is not a thing that happens; were it to, the account
/// wins.
pub async fn admit_peer(store: &Store, credential: &str, address: IpAddr) -> Result<Option<Peer>, store::Error> {
    if let Some(who) = admit(store, credential, address).await? {
        return Ok(Some(Peer::Client(who)));
    }
    admit_provider(store, credential, address).await
}

/// Whether a credential given `allowed` accepts a peer at `address`.
fn accepts(allowed: Option<IpAddr>, address: IpAddr) -> bool {
    match allowed {
        Some(allowed) => allowed.to_canonical() == address.to_canonical(),
        None => true,
    }
}
