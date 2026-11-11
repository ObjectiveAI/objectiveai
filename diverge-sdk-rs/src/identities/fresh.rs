//! A token, good for as long as asked.

use std::io;
use std::time::Duration;

use super::claims::now;
use super::record::{Record, read, write};
use super::refresh::refresh;
use super::{Error, Identities, Name, Token};
use crate::file_lock;

impl Identities {
    /// `name`'s access token, good for longer than `valid_for` from
    /// now: the one presented to a server — see the [rules](super),
    /// rule 4.
    pub async fn access_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        self.fresh(name, valid_for, |record| Some(record.access())).await
    }

    /// `name`'s ID token, good for longer than `valid_for` from now,
    /// as [`access_token`](Self::access_token) is. An identity whose
    /// issuer answered no ID token at its last refresh, nor at its
    /// login, is [`Error::NoIdToken`].
    pub async fn id_token(&self, name: &Name, valid_for: Duration) -> Result<Token, Error> {
        self.fresh(name, valid_for, Record::id).await
    }

    /// The token `pick` takes from `name`'s record, refreshed first
    /// when it is good for no longer than `valid_for`.
    async fn fresh(&self, name: &Name, valid_for: Duration, pick: fn(&Record) -> Option<Token>) -> Result<Token, Error> {
        let paths = self.paths(name);
        match tokio::fs::metadata(&paths.folder).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Err(Error::Missing(name.clone())),
            Err(source) => return Err(Error::Io { path: paths.folder, source }),
        }
        let held = file_lock::wait_exclusive(&paths.lock).await.map_err(Error::Lock)?;
        let mut record = read(&paths).await?.ok_or_else(|| Error::Missing(name.clone()))?;
        let bound = now().saturating_add(i64::try_from(valid_for.as_secs()).unwrap_or(i64::MAX));
        if let Some(token) = pick(&record).filter(|token| token.expires() > bound) {
            drop(held);
            return Ok(token);
        }
        refresh(&self.http, &mut record).await?;
        write(&paths, &record).await?;
        drop(held);
        let token = pick(&record).ok_or(Error::NoIdToken)?;
        if token.expires() <= bound {
            return Err(Error::TooShort { expires_in: token.expires().saturating_sub(now()) });
        }
        Ok(token)
    }
}
