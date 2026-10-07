//! The handshake: a credential and an address become an account, or
//! nothing.

use std::net::IpAddr;

use super::{Who, key};
use crate::store::{self, Store, accounts};

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
    if let Some(allowed) = account.address
        && allowed.to_canonical() != address.to_canonical()
    {
        return Ok(None);
    }
    Ok(Some(Who { id: account.id }))
}
