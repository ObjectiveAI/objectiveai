//! The board, in the tool's database.
//!
//! Two tables: [`schema`] makes them; [`tasks`] posts, lists, claims,
//! completes, releases and deletes; [`comments`] adds to and reads a
//! task's discussion. The tables hold one board, the database's.
//! Every state change of a task is one conditional statement, so two
//! agents racing for it never both win.

pub mod comments;
mod schema;
pub mod tasks;

use diverge_sdk::container_proxy::inside::Client;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{PgPool, Row as _};
use tokio::sync::OnceCell;

use crate::caller::Participant;
use crate::failure::Failure;

/// The pool, made and the tables ensured on the first call that needs
/// the database. A failure leaves nothing behind, so the next call
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

/// The agent a row's three columns `<prefix>_template`,
/// `<prefix>_index` and `<prefix>_name` name, or none when the
/// template column is null.
fn participant(row: &PgRow, prefix: &str) -> Result<Option<Participant>, sqlx::Error> {
    let template: Option<String> = row.try_get(format!("{prefix}_template").as_str())?;
    let Some(template) = template else {
        return Ok(None);
    };
    Ok(Some(Participant { template, index: row.try_get(format!("{prefix}_index").as_str())?, name: row.try_get(format!("{prefix}_name").as_str())? }))
}
