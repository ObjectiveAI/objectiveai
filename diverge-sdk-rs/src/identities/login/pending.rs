//! A login waiting for a person.

use std::time::Duration;

use zeroize::Zeroizing;

use crate::file_lock;
use crate::identities::claims::now;
use crate::identities::oauth::{Answer, Tokens, post};
use crate::identities::record::{Record, write};
use crate::identities::{Error, Identities, Name};

/// The seconds a `slow_down` adds to the interval, as RFC 8628 §3.5
/// requires.
const SLOWER: u64 = 5;

/// A login begun: what to show the person, and the device code the
/// wait polls with, which is never shown. It holds the identity's lock
/// from [`login`](Identities::login) on, and lets it go when it is
/// dropped — at the end of [`wait`](Self::wait), however the login
/// ended, or unwaited.
pub struct Pending {
    /// The code the person enters.
    pub user_code: String,
    /// Where the person enters it.
    pub verification_uri: String,
    /// Where the person need not enter it: the URI with the code in
    /// it, where the issuer gives one.
    pub verification_uri_complete: Option<String>,
    /// The seconds the code is good for.
    pub expires_in: u64,
    pub(super) identities: Identities,
    pub(super) name: Name,
    /// The identity's lock, held for the login's life and read by
    /// nothing: dropping it is letting it go.
    pub(super) _held: file_lock::Held,
    pub(super) issuer: String,
    pub(super) client_id: String,
    pub(super) token_endpoint: String,
    pub(super) scope: String,
    pub(super) resource: Option<String>,
    pub(super) device_code: Zeroizing<String>,
    pub(super) interval: u64,
    pub(super) expires: i64,
}

impl Pending {
    /// Poll the token endpoint until the person decides, as RFC 8628
    /// §3.5 states: every `interval` seconds, five more at each
    /// `slow_down`, on through `authorization_pending`. An approval
    /// as the identity named writes it — replacing one already there —
    /// under the lock the login holds, and answers its name; an
    /// approval as another subject is [`Error::OtherSubject`], and
    /// nothing is written; a denial is [`Error::Denied`], an expired
    /// code [`Error::Expired`], any other refusal [`Error::Refused`].
    /// The lock is let go as this returns, on every path.
    pub async fn wait(self) -> Result<Name, Error> {
        let mut interval = self.interval;
        loop {
            tokio::time::sleep(Duration::from_secs(interval)).await;
            if now() >= self.expires {
                return Err(Error::Expired);
            }
            let form = [
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("device_code", self.device_code.as_str()),
                ("client_id", self.client_id.as_str()),
            ];
            let answer: Answer<Tokens> = post(&self.identities.http, &self.token_endpoint, &form).await?;
            match answer {
                Answer::Answered(tokens) => return self.keep(tokens).await,
                Answer::Refused(refusal) => match refusal.error.as_str() {
                    "authorization_pending" => {}
                    "slow_down" => interval = interval.saturating_add(SLOWER),
                    "expired_token" => return Err(Error::Expired),
                    "access_denied" => return Err(Error::Denied),
                    _ => return Err(Error::Refused { code: refusal.error, description: refusal.error_description }),
                },
            }
        }
    }

    /// The identity the approval's tokens are, written under the lock
    /// the login holds: refused, and nothing written, when they are not
    /// the identity the login named.
    async fn keep(&self, tokens: Tokens) -> Result<Name, Error> {
        let record =
            Record::signed_in(&self.issuer, &self.client_id, &self.token_endpoint, &self.scope, self.resource.as_deref(), tokens)?;
        let name = record.name();
        if name != self.name {
            return Err(Error::OtherSubject { expected: self.name.clone(), answered: name });
        }
        write(&self.identities.paths(&name), &record).await?;
        Ok(name)
    }
}
