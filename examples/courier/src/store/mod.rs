//! The mail, in the caller's database.
//!
//! Three tables, every row carrying the template the courier was made
//! from: [`schema`] makes them; [`conversations`] opens a conversation
//! and lists a caller's; [`messages`] adds a message and reads a
//! conversation's, moving the reader's read mark. A message is unread
//! for a participant while its id is above that participant's mark,
//! and a participant's own messages move their own mark, so they are
//! read for them from the start.

pub mod conversations;
pub mod messages;
mod schema;

use diverge_sdk::container_proxy::inside::Client;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::OnceCell;

use crate::failure::Failure;

/// The pool, made and the tables ensured on the first call that
/// needs the database. A failure leaves nothing behind, so the next
/// call tries again.
#[derive(Default)]
pub struct Store {
    pool: OnceCell<PgPool>,
}

impl Store {
    /// The pool, through the proxy's loopback pgwire.
    pub async fn pool(&self, client: &Client) -> Result<&PgPool, Failure> {
        self.pool
            .get_or_try_init(|| async {
                let pool = PgPoolOptions::new().max_connections(4).connect(&client.postgres_url()).await?;
                schema::ensure(&pool).await?;
                Ok::<_, Failure>(pool)
            })
            .await
    }
}
