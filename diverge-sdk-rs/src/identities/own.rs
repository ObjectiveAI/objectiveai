//! A program's own side of the handshake.

use std::path::Path;
use std::time::Duration;

use std::sync::Arc;

use super::claims::claims;
use super::{Error, Identities, KeySets, Name};
use crate::wire::handshake::{Judged, Peer, Presented};

/// How long a token presented in a handshake is good for, at least:
/// one that would expire sooner is refreshed first.
pub const VALID_FOR: Duration = Duration::from_secs(60);

/// A program's own identity — the one it is, whose token it presents
/// on every connection it accepts — and the cache it verifies other
/// ends' tokens by.
///
/// What a daemon's or a provider's
/// [`Authenticator`](crate::wire::handshake::Authenticator) is built
/// on: [`present`](Self::present) is its half of a handshake, and
/// [`claimed`](Self::claimed) and [`verify_from`](Self::verify_from)
/// are the parts of its judgment every program shares, between which
/// each asks whom it admits. The rules:
///
/// 1. **Presenting.** The token presented is the OpenID Connect ID
///    token of the identity `own` — the one token OpenID Connect makes
///    a signed JWT naming its issuer and its subject, which an access
///    token need not be — good for longer than [`VALID_FOR`] —
///    refreshed first where it is not — and with it the identity's key
///    on this host, which the token names and the handshake signs
///    with. A program with no identity presents nothing, and so
///    completes no handshake it answers.
/// 2. **Trusting.** There is no list of issuers. A program reads whom
///    a token claims to be — [`claimed`](Self::claimed), unsigned —
///    and decides whether it could admit that identity at all; only
///    then is the token's issuer trusted, for that token alone, and
///    its key set fetched. A token claiming an identity the program
///    would not admit makes it fetch nothing.
/// 3. **Verifying.** Another end's ID token is verified by the
///    [`KeySets`], trusting the one issuer rule 2 arrived at — an
///    access token refused — with no audience asked for; its
///    `(issuer, subject)` is the [`Peer`] answered, with the key the
///    token names, which the handshake holds the other end to. A token
///    that names no key is refused.
#[derive(Debug, Clone)]
pub struct Own {
    identities: Identities,
    own: Option<Peer>,
    key_sets: KeySets,
}

impl Own {
    /// The program's identities at `root`, `own` the one it is — or
    /// none — verifying by `key_sets`.
    pub async fn new(root: &Path, own: Option<Peer>, key_sets: KeySets) -> Result<Own, Error> {
        Ok(Own { identities: Identities::open(root).await?, own, key_sets })
    }

    /// Every identity on the host, as the program opened them: what a
    /// listing of them reads.
    pub fn identities(&self) -> &Identities {
        &self.identities
    }

    /// The identity this program is, where it has one.
    pub fn identity(&self) -> Option<&Peer> {
        self.own.as_ref()
    }

    /// This program's token and its key — see rule 1. The answer's
    /// error is a reason for a log.
    pub async fn present(&self) -> Result<Presented, String> {
        let own = self.own.as_ref().ok_or_else(|| "this program has no identity".to_string())?;
        let name = Name { issuer: own.issuer.clone(), subject: own.subject.clone() };
        self.present_as(&name).await
    }

    /// The ID token of the identity `name`, one of the host's, good for
    /// longer than [`VALID_FOR`], and that identity's key: what is
    /// presented to a remote daemon as the account of ours a record of
    /// it names, whatever `own` is. The answer's error is a reason for
    /// a log.
    pub async fn present_as(&self, name: &Name) -> Result<Presented, String> {
        let (token, key) = self.identities.presentation(name, VALID_FOR).await.map_err(|error| error.to_string())?;
        Ok(Presented { token: token.expose().to_string(), signer: Arc::new(key) })
    }

    /// Whom another end's `token` claims to be, its signature unread —
    /// see rule 2. Nothing is fetched, and nothing is established: the
    /// answer is only what to ask about before
    /// [`verify_from`](Self::verify_from). The answer's error is a
    /// reason for a log.
    pub fn claimed(&self, token: &str) -> Result<Peer, String> {
        let read = claims(token).ok_or_else(|| "the other end's token is no JWT".to_string())?;
        match (read.iss, read.sub) {
            (Some(issuer), Some(subject)) => Ok(Peer { issuer, subject }),
            _ => Err("the other end's token names no issuer or no subject".to_string()),
        }
    }

    /// Another end's `token`, verified as rule 3 verifies one, trusting
    /// `issuer` alone. The answer's error is a reason for a log.
    pub async fn verify_from(&self, token: &str, issuer: &str) -> Result<Judged, String> {
        let verified = self
            .key_sets
            .verify(token, |trusted| trusted == issuer, None)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Judged { peer: Peer { issuer: verified.issuer, subject: verified.subject }, key: verified.key })
    }
}
