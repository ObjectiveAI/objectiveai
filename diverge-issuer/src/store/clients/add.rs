//! Adding a client.

use super::{Clients, New};
use crate::store::{Error, names};

impl Clients<'_> {
    /// Add `client`, once it passes [`New::check`]. An id already held
    /// is [`Error::Exists`]. The client and its grants, audiences and
    /// redirect URIs are written in one transaction.
    pub async fn add(&self, client: &New) -> Result<(), Error> {
        client.check()?;
        let mut transaction = self.store.pool.begin().await?;
        let added = sqlx::query("insert into clients (id, kind, secret_hash, made) values (?, ?, ?, ?)")
            .bind(&client.id)
            .bind(client.kind.name())
            .bind(&client.secret_hash)
            .bind(crate::now())
            .execute(&mut *transaction)
            .await;
        match added {
            Ok(_) => {}
            Err(error) if names::taken(&error) => return Err(Error::Exists(client.id.clone())),
            Err(error) => return Err(Error::Query(error)),
        }
        for grant in &client.grants {
            sqlx::query("insert into client_grants (client, name) values (?, ?)")
                .bind(&client.id)
                .bind(grant.name())
                .execute(&mut *transaction)
                .await?;
        }
        for (position, audience) in (0_i64..).zip(&client.audiences) {
            sqlx::query("insert into client_audiences (client, position, audience) values (?, ?, ?)")
                .bind(&client.id)
                .bind(position)
                .bind(audience)
                .execute(&mut *transaction)
                .await?;
        }
        for redirect in &client.redirects {
            sqlx::query("insert into client_redirects (client, uri) values (?, ?)")
                .bind(&client.id)
                .bind(redirect)
                .execute(&mut *transaction)
                .await?;
        }
        transaction.commit().await?;
        Ok(())
    }
}
