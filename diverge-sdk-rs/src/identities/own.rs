//! A program's own side of the handshake.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use super::{Error, Identities, KeySets, Name};
use crate::wire::handshake::Peer;

/// How long a token presented in a handshake is good for, at least:
/// one that would expire sooner is refreshed first.
pub const VALID_FOR: Duration = Duration::from_secs(60);

/// A program's own identities — one subject per issuer, the one whose
/// token it presents for that issuer in every case — and the cache it
/// verifies other ends' tokens by.
///
/// What a daemon's or a provider's
/// [`Authenticator`](crate::wire::handshake::Authenticator) is built
/// on: [`present`](Self::present) is its half of every handshake, and
/// [`verify`](Self::verify) is the part of its judgment every program
/// shares, to which each adds whom it admits. The rules:
///
/// 1. **Presenting.** The token presented for an issuer is the OpenID
///    Connect ID token of the identity `own` names there — the one
///    token OpenID Connect makes a signed JWT naming its issuer and its
///    subject, which an access token need not be — good for longer than
///    [`VALID_FOR`] — refreshed first where it is not. An issuer `own`
///    names no subject at is refused.
/// 2. **Trusting.** An issuer is trusted exactly when `own` names a
///    subject there. The connectee answers from the connector's issuer,
///    so an issuer this program holds no identity at could never
///    complete a handshake, and no key set is fetched for one.
/// 3. **Verifying.** Another end's ID token is verified by the
///    [`KeySets`] under rule 2 — an access token refused — with no
///    audience asked for; its `(issuer, subject)` is the [`Peer`]
///    answered.
#[derive(Debug, Clone)]
pub struct Own {
    identities: Identities,
    own: BTreeMap<String, String>,
    key_sets: KeySets,
}

impl Own {
    /// The program's identities at `root`, `own` mapping each issuer to
    /// the subject it presents there, verifying by `key_sets`.
    pub async fn new(root: &Path, own: BTreeMap<String, String>, key_sets: KeySets) -> Result<Own, Error> {
        Ok(Own { identities: Identities::open(root).await?, own, key_sets })
    }

    /// Every identity on the host, as the program opened them: what a
    /// listing of them reads.
    pub fn identities(&self) -> &Identities {
        &self.identities
    }

    /// The subject this program presents at `issuer`, where it has one.
    pub fn subject(&self, issuer: &str) -> Option<&str> {
        self.own.get(issuer).map(String::as_str)
    }

    /// Every identity this program presents: one per issuer it holds a
    /// subject at.
    pub fn presented(&self) -> impl Iterator<Item = crate::shared::Identity> + '_ {
        self.own.iter().map(|(issuer, subject)| crate::shared::Identity { issuer: issuer.clone(), subject: subject.clone() })
    }

    /// This program's token from `issuer` — see rule 1. The answer's
    /// error is a reason for a log.
    pub async fn present(&self, issuer: &str) -> Result<String, String> {
        let subject = self.subject(issuer).ok_or_else(|| format!("this program holds no identity at `{issuer}`"))?;
        let name = Name { issuer: issuer.to_string(), subject: subject.to_string() };
        let token = self.identities.id_token(&name, VALID_FOR).await.map_err(|error| error.to_string())?;
        Ok(token.expose().to_string())
    }

    /// The ID token of the identity `name`, one of the host's, good for
    /// longer than [`VALID_FOR`]: what is presented to a remote daemon
    /// as the account of ours a record of it names, whichever subject
    /// `own` names at that issuer. The answer's error is a reason for a
    /// log.
    pub async fn present_as(&self, name: &Name) -> Result<String, String> {
        let token = self.identities.id_token(name, VALID_FOR).await.map_err(|error| error.to_string())?;
        Ok(token.expose().to_string())
    }

    /// Another end's `token`, verified as rule 3 verifies one, trusting
    /// `issuer` alone: what a remote daemon's answer is judged by, its
    /// issuer the record's whether or not `own` names a subject there.
    pub async fn verify_from(&self, token: &str, issuer: &str) -> Result<Peer, String> {
        let verified = self
            .key_sets
            .verify(token, |trusted| trusted == issuer, None)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Peer { issuer: verified.issuer, subject: verified.subject })
    }

    /// Another end's `token`, verified — see rules 2 and 3. The
    /// answer's error is a reason for a log.
    pub async fn verify(&self, token: &str) -> Result<Peer, String> {
        let verified = self
            .key_sets
            .verify(token, |issuer| self.own.contains_key(issuer), None)
            .await
            .map_err(|error| error.to_string())?;
        Ok(Peer { issuer: verified.issuer, subject: verified.subject })
    }
}
