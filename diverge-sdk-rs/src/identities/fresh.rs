//! A token, good for as long as asked.

use std::io;
use std::time::Duration;

use super::claims::now;
use super::key::{self, Key};
use super::record::{Record, read, write};
use super::refresh::refresh;
use super::{Error, Identities, Name, Token};
use crate::file_lock;

impl Identities {
    /// `name`'s access token, good for longer than `valid_for` from
    /// now, and bound to the identity's key on this host: the one
    /// presented — see the [rules](super), rule 4.
    pub async fn access_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        Ok(self.presentation(name, valid_for).await?.0)
    }

    /// `name`'s ID token, good for longer than `valid_for` from now,
    /// as [`access_token`](Self::access_token) is. An identity whose
    /// issuer answered no ID token at its last refresh, nor at its
    /// login, is [`Error::NoIdToken`]. It says who the identity is, to
    /// this host, and is presented to nobody.
    pub async fn id_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        Ok(self.fresh(name, valid_for, Record::id, false).await?.0)
    }

    /// What `name` presents in a handshake: its access token, good
    /// for longer than `valid_for` from now, and the key that token is
    /// bound to — read together, under the identity's lock, so that
    /// the one is the other's. The rules:
    ///
    /// 1. **The key first.** The identity's key is read, and made
    ///    where it has none — see [`Key`], rule 2.
    /// 2. **A token of another key is no token.** An access token held
    ///    that is bound to another key, or to none — one from before
    ///    the identity had a key — is refreshed, whatever its expiry.
    /// 3. **An issuer that binds no key cannot be presented.** A
    ///    refresh whose access token is still not bound to the key —
    ///    it is no JWT, or carries no `cnf.jkt`, or another's — is
    ///    [`Error::Unbound`]: the issuer does not bind its tokens as
    ///    RFC 9449 binds them, and no Diverge program takes a token
    ///    bound to no key.
    pub async fn presentation(&self, name: &Name, valid_for: Duration) -> Result<(Token, Key), Error> {
        self.fresh(name, valid_for, |record| Some(record.access()), true).await
    }

    /// The token `pick` takes from `name`'s record, and the identity's
    /// key, the token refreshed first when it is good for no longer
    /// than `valid_for` — or, where `bound` is set, when the access
    /// token held is not bound to the key.
    async fn fresh(&self, name: &Name, valid_for: Duration, pick: fn(&Record) -> Option<Token>, bound: bool) -> Result<(Token, Key), Error> {
        let paths = self.paths(name);
        match tokio::fs::metadata(&paths.folder).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Error::Missing(name.clone())),
            Err(source) => return Err(Error::Io { path: paths.folder, source }),
        }
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        let mut record = read(&paths).await?.ok_or_else(|| Error::Missing(name.clone()))?;
        let key = key::ensure(&paths).await?;
        let thumbprint = key.thumbprint();
        let until = now().saturating_add(i64::try_from(valid_for.as_secs()).unwrap_or(i64::MAX));
        if !bound || record.bound(&thumbprint) {
            if let Some(token) = pick(&record).filter(|token| token.expires() > until) {
                drop(held);
                return Ok((token, key));
            }
        }
        refresh(&self.http, &mut record, &key).await?;
        write(&paths, &record).await?;
        drop(held);
        let token = pick(&record).ok_or(Error::NoIdToken)?;
        if bound && !record.bound(&thumbprint) {
            return Err(Error::Unbound(name.clone()));
        }
        if token.expires() <= until {
            return Err(Error::TooShort { expires_in: token.expires().saturating_sub(now()) });
        }
        Ok((token, key))
    }
}
