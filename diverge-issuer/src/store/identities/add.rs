//! Adding an identity.

use super::Identities;
use crate::store::{Error, names};

impl Identities<'_> {
    /// Add `subject`, signing in with the password `password_hash` is
    /// the argon2id hash of — see [`password`](crate::password). A
    /// subject that is not one is [`Error::Invalid`]; one already
    /// held is [`Error::Exists`].
    pub async fn add(&self, subject: &str, password_hash: &str) -> Result<(), Error> {
        names::subject(subject)?;
        let added = sqlx::query("insert into identities (subject, password_hash, made) values (?, ?, ?)")
            .bind(subject)
            .bind(password_hash)
            .bind(crate::now())
            .execute(&self.store.pool)
            .await;
        match added {
            Ok(_) => Ok(()),
            Err(error) if names::taken(&error) => Err(Error::Exists(subject.to_string())),
            Err(error) => Err(Error::Query(error)),
        }
    }
}
