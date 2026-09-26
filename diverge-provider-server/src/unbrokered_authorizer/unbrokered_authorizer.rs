//! The authorizer.

use std::net::IpAddr;
use std::path::PathBuf;

use diverge_provider_sdk::server::unbrokered_authorizer;
use futures_util::future;

use super::Error;
use crate::config::auth::{Auth, Hook, HookInput, HookOutput, Key, Unbrokered};
use crate::hook;

/// The provider's authorizer: the ways the `auth` section lists, and
/// the `hooks/` directory the hooks among them are found in.
#[derive(Debug)]
pub struct UnbrokeredAuthorizer {
    /// The ways, in the configuration's order. Empty is a provider
    /// that accepts no dialling peer.
    ways: Vec<Unbrokered>,
    /// The `hooks/` directory of the provider.
    hooks_dir: PathBuf,
}

/// What one way said of a credential.
enum Judgement {
    /// Accepted, and this is who presented it.
    Accepted(String),
    /// Refused.
    Refused,
    /// The hook, by name, did not answer at all, which refuses.
    Failed(String, hook::Error),
}

impl UnbrokeredAuthorizer {
    /// An authorizer over the `auth` section, absent or present, with
    /// the provider's `hooks/` directory.
    pub fn new(auth: Option<Auth>, hooks_dir: PathBuf) -> Self {
        UnbrokeredAuthorizer {
            ways: auth.and_then(|auth| auth.unbrokered).unwrap_or_default(),
            hooks_dir,
        }
    }

    /// One way's judgement of the credential.
    async fn judge(&self, way: &Unbrokered, credential: &str, address: IpAddr) -> Judgement {
        match way {
            Unbrokered::Key(key) => Self::key(key, credential, address),
            Unbrokered::Hook(hook) => self.hook(hook, credential, address).await,
        }
    }

    /// A key: the credential equals it, and the address is the key's
    /// where the key names one.
    fn key(key: &Key, credential: &str, address: IpAddr) -> Judgement {
        if key.address.is_some_and(|allowed| allowed != address) {
            return Judgement::Refused;
        }
        if !equal(key.key.as_bytes(), credential.as_bytes()) {
            return Judgement::Refused;
        }
        Judgement::Accepted(key.identity.clone())
    }

    /// A hook: run with the credential and the address, its answer
    /// taken as written, and no answer a failure.
    async fn hook(&self, hook: &Hook, credential: &str, address: IpAddr) -> Judgement {
        let input = HookInput {
            credential: credential.to_string(),
            address,
        };
        match hook::run::<HookInput, HookOutput>(&self.hooks_dir, &hook.authorize_hook, &input).await {
            Ok(HookOutput::Authorized { identity }) => Judgement::Accepted(identity),
            Ok(HookOutput::Refused) => Judgement::Refused,
            Err(error) => Judgement::Failed(hook.authorize_hook.clone(), error),
        }
    }
}

/// Whether two byte strings are equal, in time that depends on their
/// length and not on where they differ: a bearer key must not leak
/// by how long the comparison took. The lengths are compared first,
/// and then every byte is folded in whether or not an earlier one
/// already differed.
fn equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl unbrokered_authorizer::UnbrokeredAuthorizer for UnbrokeredAuthorizer {
    type Error = Error;

    /// Every way asked at once; the first in the configuration's
    /// order that accepts decides; none accepting is the refusal,
    /// with the hooks that did not answer.
    async fn authorize(&self, credential: &str, address: IpAddr) -> Result<String, Error> {
        let judgements = future::join_all(self.ways.iter().map(|way| self.judge(way, credential, address))).await;
        let mut failed = Vec::new();
        for judgement in judgements {
            match judgement {
                Judgement::Accepted(identity) => return Ok(identity),
                Judgement::Refused => {}
                Judgement::Failed(name, error) => failed.push((name, error)),
            }
        }
        Err(Error::Refused { failed })
    }
}
