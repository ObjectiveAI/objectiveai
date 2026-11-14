//! One identity presented, and the other end held to its key.

use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::BoxFuture;

use super::{Identities, KeySets, Name, VALID_FOR};
use crate::wire::handshake::{Authenticator, Judged, Peer, Presented};

/// How long a key set is held where its issuer names no `max-age`.
const KEYS_FOR: Duration = Duration::from_secs(60 * 60);

/// The handshake's side of a program that dials as one identity and
/// admits nobody — a command line, an app — see the [rules](super),
/// rule 11:
///
/// 1. **Presenting.** The identity's access token, good for at least
///    [`VALID_FOR`], refreshed as rule 4 says, and the identity's key
///    on this host, which the token is bound to.
/// 2. **Verifying.** The other end's token is verified against its
///    issuer's keys, and must be bound to a key; the handshake then
///    holds the other end to that key. So the other end is whoever its
///    token says, and not merely whoever holds a copy of the token.
/// 3. **Expecting.** Where the program was told which identity to
///    expect — see [`expecting`](Self::expecting) — the token must be
///    that identity's: its issuer is the only one trusted, and nothing
///    is fetched for a token claiming another. A token of any other
///    identity fails the handshake.
/// 4. **Expecting nobody.** Where it was told none, any identity
///    whose token verifies is taken, whatever its issuer: the program
///    knows it reached someone who is who they say, and not that it
///    reached whom it meant. An address the program does not trust
///    wants an identity to expect.
#[derive(Debug, Clone)]
pub struct Presenter {
    identities: Identities,
    name: Name,
    key_sets: KeySets,
    expected: Option<Peer>,
}

impl Presenter {
    /// `name`, presented from `identities`, expecting nobody.
    pub fn new(identities: Identities, name: Name) -> Presenter {
        let key_sets = KeySets::with(identities.http.clone(), KEYS_FOR);
        Presenter { identities, name, key_sets, expected: None }
    }

    /// This presenter, expecting `expected` at the other end — rule 3
    /// — or nobody, rule 4.
    pub fn expecting(mut self, expected: Option<Peer>) -> Presenter {
        self.expected = expected;
        self
    }
}

impl Authenticator for Presenter {
    fn present<'a>(&'a self) -> BoxFuture<'a, Result<Presented, String>> {
        Box::pin(async move {
            let (token, key) = self.identities.presentation(&self.name, VALID_FOR).await.map_err(|error| error.to_string())?;
            Ok(Presented { token: token.expose().to_string(), key: key.public(), signer: Arc::new(key) })
        })
    }

    fn verify<'a>(&'a self, token: &'a str, _address: IpAddr) -> BoxFuture<'a, Result<Judged, String>> {
        Box::pin(async move {
            let trusted = |issuer: &str| self.expected.as_ref().is_none_or(|expected| expected.issuer == issuer);
            let verified = self.key_sets.verify(token, trusted, None).await.map_err(|error| error.to_string())?;
            let peer = Peer { issuer: verified.issuer, subject: verified.subject };
            if self.expected.as_ref().is_some_and(|expected| *expected != peer) {
                return Err("the other end is not the identity expected there".to_string());
            }
            Ok(Judged { peer, thumbprint: verified.thumbprint })
        })
    }
}
