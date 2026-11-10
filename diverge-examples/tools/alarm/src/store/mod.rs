//! The alarms, in the tool's database.
//!
//! One table: [`schema`] makes it; [`alarms`] sets, lists, cancels,
//! and — for the scheduler — finds the due and settles them. Every
//! alarm names the agent it wakes, so however many agents share the
//! database, each sees and wakes to its own alone.

pub mod alarms;
mod schema;

use diverge_sdk::container_proxy::inside::Client;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::OnceCell;

use crate::failure::Failure;

/// The pool, made and the table ensured on the first use that needs
/// the database. A failure leaves nothing behind, so the next use
/// tries again.
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
