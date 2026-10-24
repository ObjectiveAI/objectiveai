//! The identity's file.

use std::io;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt as _;

use super::claims::{claims, now};
use super::name::Paths;
use super::oauth::Tokens;
use super::{Error, Name, Token};

/// Everything an identity's file holds: who it is, how it refreshes,
/// and the tokens it holds now. No type outside this module has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Record {
    /// The issuer identifier.
    pub(super) issuer: String,
    /// The subject.
    pub(super) subject: String,
    /// The client the identity signed in through.
    pub(super) client_id: String,
    /// The issuer's token endpoint, from its discovery document at the
    /// login.
    pub(super) token_endpoint: String,
    /// The scope granted.
    pub(super) scope: String,
    /// The audience asked for at the login, asked again at every
    /// refresh.
    pub(super) resource: Option<String>,
    /// The refresh token: the newest the issuer answered.
    pub(super) refresh_token: String,
    /// The access token.
    pub(super) access_token: String,
    /// When the access token expires, in seconds since the Unix epoch.
    pub(super) access_expires: i64,
    /// The ID token, where the issuer answered one.
    pub(super) id_token: Option<String>,
    /// When the ID token expires.
    pub(super) id_expires: Option<i64>,
}

impl Record {
    /// The identity it is.
    pub(super) fn name(&self) -> Name {
        Name { issuer: self.issuer.clone(), subject: self.subject.clone() }
    }

    /// The access token, as handed out.
    pub(super) fn access(&self) -> Token {
        Token::new(&self.access_token, self.access_expires)
    }

    /// The ID token, as handed out, where there is one.
    pub(super) fn id(&self) -> Option<Token> {
        Some(Token::new(self.id_token.as_deref()?, self.id_expires?))
    }

    /// The record `answer` makes for a new sign-in: the issuer's
    /// answer to a login's token request. The subject is the ID
    /// token's `sub`; an answer with no refresh token is
    /// [`Error::NoRefreshToken`].
    pub(super) fn signed_in(issuer: &str, client_id: &str, token_endpoint: &str, scope: &str, resource: Option<&str>, answer: Tokens) -> Result<Record, Error> {
        let subject = answer.id_token.as_deref().and_then(claims).and_then(|claims| claims.sub).ok_or(Error::Subject)?;
        let refresh_token = answer.refresh_token.clone().ok_or(Error::NoRefreshToken)?;
        let mut record = Record {
            issuer: issuer.to_string(),
            subject,
            client_id: client_id.to_string(),
            token_endpoint: token_endpoint.to_string(),
            scope: scope.to_string(),
            resource: resource.map(str::to_string),
            refresh_token,
            access_token: String::new(),
            access_expires: 0,
            id_token: None,
            id_expires: None,
        };
        record.take(answer)?;
        Ok(record)
    }

    /// The tokens of `answer` taken in: the access token, and its
    /// expiry from `expires_in`, else from its own `exp`, else
    /// [`Error::NoExpiry`]; the refresh token where the answer rotated
    /// it; the ID token and the scope where the answer has them.
    pub(super) fn take(&mut self, answer: Tokens) -> Result<(), Error> {
        let expires = match answer.expires_in {
            Some(seconds) => now().saturating_add(i64::try_from(seconds).unwrap_or(i64::MAX)),
            None => claims(&answer.access_token).and_then(|claims| claims.exp).ok_or(Error::NoExpiry)?,
        };
        self.access_token = answer.access_token;
        self.access_expires = expires;
        if let Some(refresh_token) = answer.refresh_token {
            self.refresh_token = refresh_token;
        }
        if let Some(id_token) = answer.id_token {
            self.id_expires = claims(&id_token).and_then(|claims| claims.exp);
            self.id_token = Some(id_token);
        }
        if let Some(scope) = answer.scope {
            self.scope = scope;
        }
        Ok(())
    }
}

/// The record at `paths`, or `None` when there is no such file. The
/// caller holds the lock.
pub(super) async fn read(paths: &Paths) -> Result<Option<Record>, Error> {
    let bytes = match tokio::fs::read(&paths.record).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => return Err(Error::Io { path: paths.record.clone(), source }),
    };
    let record = serde_json::from_slice(&bytes).map_err(|source| Error::Json { path: paths.record.clone(), source })?;
    Ok(Some(record))
}

/// Write `record` at `paths`: whole, to the staging file — made
/// owner-only — and renamed over the record, so a reader finds the old
/// file or the new one and never a part of either. The caller holds
/// the lock, and the folder exists.
pub(super) async fn write(paths: &Paths, record: &Record) -> Result<(), Error> {
    let staging = &paths.staging;
    let failed = |source| Error::Io { path: staging.clone(), source };
    let bytes = serde_json::to_vec_pretty(record).map_err(|source| Error::Json { path: staging.clone(), source })?;
    let mut options = tokio::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(staging).await.map_err(failed)?;
    file.write_all(&bytes).await.map_err(failed)?;
    file.sync_all().await.map_err(failed)?;
    drop(file);
    tokio::fs::rename(staging, &paths.record).await.map_err(|source| Error::Io { path: paths.record.clone(), source })
}
