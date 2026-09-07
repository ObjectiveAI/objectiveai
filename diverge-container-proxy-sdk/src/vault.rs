//! The vault: keys the caller holds, read, written and locked from
//! inside the container.
//!
//! Each method is one `POST` to the proxy's `/vault/agent/<op>`, its
//! body the wire's request payload and its answer the wire's one
//! message, decoded here so a caller sees values and errors rather
//! than frames. Keys are whatever strings the program chooses; which
//! vault they land in is the caller's decision, made from what it
//! knows about this container.
//!
//! # The lock
//!
//! [`vault_lock`](Client::vault_lock) returns once the lock is HELD,
//! however long that takes, and the lock lasts `ttl` seconds from
//! then. The holder is this container: locking a key it already
//! holds refreshes the TTL and returns at once, which is how a long
//! job keeps its lock — lock again before the TTL runs out. Expiry
//! releases silently; [`vault_unlock`](Client::vault_unlock) releases
//! early, and unlocking a key this container does not hold is an
//! error. A TTL of `0` is refused.
//!
//! # No retry
//!
//! None of these is safe to repeat blindly, so a call whose answer
//! never came fails as [`Error::VaultStatus`] and the program
//! decides.

use bytes::Bytes;
use diverge_provider_sdk::container_proxy::vault;
use diverge_provider_sdk::encode::{Encode, Writer};

use crate::{Client, Error};

impl Client {
    /// Read a key: its value, or [`None`] when there is no such key.
    pub async fn vault_get(&self, key: &str) -> Result<Option<Bytes>, Error> {
        let body = encode(&vault::get::request::Request { key });
        let answer = self.post("get", body).await?;
        match vault::get::response::Frame::decode(&answer)
            .map_err(Error::VaultAnswer)?
        {
            vault::get::response::Frame::Present(value) => {
                Ok(Some(answer.slice_ref(value)))
            }
            vault::get::response::Frame::Missing => Ok(None),
            vault::get::response::Frame::Error(message) => {
                Err(Error::Vault(message.to_owned()))
            }
        }
    }

    /// Write a key, creating or replacing it. Empty is a value.
    pub async fn vault_set(&self, key: &str, value: &[u8]) -> Result<(), Error> {
        let mut body = Vec::new();
        vault::set::request::Request { key, value }
            .encode(&mut Writer::new(&mut body))
            .map_err(Error::VaultKey)?;
        let answer = self.post("set", body).await?;
        done(&answer)
    }

    /// Remove a key. Succeeds whether or not it existed.
    pub async fn vault_delete(&self, key: &str) -> Result<(), Error> {
        let body = encode(&vault::delete::request::Request { key });
        let answer = self.post("delete", body).await?;
        done(&answer)
    }

    /// Hold a key's lock for `ttl` seconds, waiting for it. Whose the
    /// lock is and how it ends is stated at the top of `vault.rs`.
    pub async fn vault_lock(&self, key: &str, ttl: u32) -> Result<(), Error> {
        let body = encode(&vault::lock::request::Request { key, ttl });
        let answer = self.post("lock", body).await?;
        done(&answer)
    }

    /// Release a key's lock early.
    pub async fn vault_unlock(&self, key: &str) -> Result<(), Error> {
        let body = encode(&vault::unlock::request::Request { key });
        let answer = self.post("unlock", body).await?;
        done(&answer)
    }

    /// One `POST` to `/vault/agent/<op>`: the answer's bytes on `200`,
    /// the status otherwise.
    async fn post(&self, op: &str, body: Vec<u8>) -> Result<Bytes, Error> {
        let response = self
            .http()
            .post(crate::url(&format!("/vault/agent/{op}")))
            .body(body)
            .send()
            .await
            .map_err(Error::VaultRequest)?;
        let status = response.status();
        if !status.is_success() {
            return Err(Error::VaultStatus(status.as_u16()));
        }
        response.bytes().await.map_err(Error::VaultRequest)
    }
}

/// Encode a request whose encoding cannot fail.
fn encode<R>(request: &R) -> Vec<u8>
where
    R: Encode<Error = std::convert::Infallible>,
{
    let mut body = Vec::new();
    request
        .encode(&mut Writer::new(&mut body))
        .unwrap_or_else(|error| match error {});
    body
}

/// Read the common answer: done, or the caller's refusal.
fn done(answer: &[u8]) -> Result<(), Error> {
    match vault::response::Frame::decode(answer).map_err(Error::VaultAnswer)? {
        vault::response::Frame::Ok => Ok(()),
        vault::response::Frame::Error(message) => {
            Err(Error::Vault(message.to_owned()))
        }
    }
}
