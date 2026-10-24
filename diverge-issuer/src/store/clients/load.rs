//! Reading a client.

use super::{Client, Clients, Grant, Kind};
use crate::store::Error;

impl Clients<'_> {
    /// The client `id`, whole, or `None` when there is no such client.
    pub async fn load(&self, id: &str) -> Result<Option<Client>, Error> {
        let pool = &self.store.pool;
        let row: Option<(String, Option<String>, i64)> = sqlx::query_as("select kind, secret_hash, made from clients where id = ?")
            .bind(id)
            .fetch_optional(pool)
            .await?;
        let Some((kind, secret_hash, made)) = row else {
            return Ok(None);
        };
        let kind = Kind::from_name(&kind).ok_or_else(|| Error::Invalid(format!("the client `{id}` has the kind `{kind}`")))?;
        let grants: Vec<String> = sqlx::query_scalar("select name from client_grants where client = ? order by name")
            .bind(id)
            .fetch_all(pool)
            .await?;
        let grants = grants
            .iter()
            .map(|name| Grant::from_name(name).ok_or_else(|| Error::Invalid(format!("the client `{id}` has the grant `{name}`"))))
            .collect::<Result<Vec<_>, _>>()?;
        let audiences = sqlx::query_scalar("select audience from client_audiences where client = ? order by position")
            .bind(id)
            .fetch_all(pool)
            .await?;
        let redirects = sqlx::query_scalar("select uri from client_redirects where client = ? order by uri")
            .bind(id)
            .fetch_all(pool)
            .await?;
        Ok(Some(Client { id: id.to_string(), kind, secret_hash, grants, audiences, redirects, made }))
    }
}
