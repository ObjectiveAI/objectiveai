//! Every client.

use super::{Client, Clients};
use crate::store::Error;

impl Clients<'_> {
    /// Every client, whole, by id.
    pub async fn list(&self) -> Result<Vec<Client>, Error> {
        let ids: Vec<String> = sqlx::query_scalar("select id from clients order by id").fetch_all(&self.store.pool).await?;
        let mut clients = Vec::with_capacity(ids.len());
        // One after another: a listing is the CLI's, never a
        // request's, and a host's clients are few.
        for id in &ids {
            if let Some(client) = self.load(id).await? {
                clients.push(client);
            }
        }
        Ok(clients)
    }
}
