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
    /// now: the one presented to a server — see the [rules](super),
    /// rule 4.
    pub async fn access_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        Ok(self.fresh(name, valid_for, |record| Some(record.access()), false).await?.0)
    }

    /// `name`'s ID token, good for longer than `valid_for` from now,
    /// as [`access_token`](Self::access_token) is, and naming the
    /// identity's key on this host. An identity whose issuer answered
    /// no ID token at its last refresh, nor at its login, is
    /// [`Error::NoIdToken`].
    pub async fn id_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        Ok(self.presentation(name, valid_for).await?.0)
    }

    /// What `name` presents in a handshake: its ID token, good for
    /// longer than `valid_for` from now, and the key that token names
    /// — read together, under the identity's lock, so that the one is
    /// the other's. The rules:
    ///
    /// 1. **The key first.** The identity's key is read, and made
    ///    where it has none — see [`Key`], rule 2.
    /// 2. **A token of another key is no token.** An ID token held
    ///    that names another key, or none — one from before the
    ///    identity had a key, or before its key was replaced — is
    ///    refreshed, whatever its expiry.
    /// 3. **An issuer that names no key cannot be presented.** A
    ///    refresh whose ID token still does not name the key is
    ///    [`Error::Unbound`]: the issuer does not write the `cnf`
    ///    claim, and no Diverge program takes a token without one.
    pub async fn presentation(&self, name: &Name, valid_for: Duration) -> Result<(Token, Key), Error> {
        self.fresh(name, valid_for, Record::id, true).await
    }

    /// The token `pick` takes from `name`'s record, and the identity's
    /// key, the token refreshed first when it is good for no longer
    /// than `valid_for` — or, where `named` is set, when the ID token
    /// held does not name the key.
    async fn fresh(&self, name: &Name, valid_for: Duration, pick: fn(&Record) -> Option<Token>, named: bool) -> Result<(Token, Key), Error> {
        let paths = self.paths(name);
        match tokio::fs::metadata(&paths.folder).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Error::Missing(name.clone())),
            Err(source) => return Err(Error::Io { path: paths.folder, source }),
        }
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        let mut record = read(&paths).await?.ok_or_else(|| Error::Missing(name.clone()))?;
        let key = key::ensure(&paths).await?;
        let public = key.public();
        let bound = now().saturating_add(i64::try_from(valid_for.as_secs()).unwrap_or(i64::MAX));
        if !named || record.names(&public) {
            if let Some(token) = pick(&record).filter(|token| token.expires() > bound) {
                drop(held);
                return Ok((token, key));
            }
        }
        refresh(&self.http, &mut record, &key.jwk()).await?;
        write(&paths, &record).await?;
        drop(held);
        let token = pick(&record).ok_or(Error::NoIdToken)?;
        if named && !record.names(&public) {
            return Err(Error::Unbound(name.clone()));
        }
        if token.expires() <= bound {
            return Err(Error::TooShort { expires_in: token.expires().saturating_sub(now()) });
        }
        Ok((token, key))
    }
}
