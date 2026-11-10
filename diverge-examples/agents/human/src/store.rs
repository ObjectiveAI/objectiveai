//! The transcript, kept: every chunk the agent's runs were made of, in
//! the agent's own database, so a page opened later — or after the
//! container started again — shows the whole of it.

use diverge_sdk::container_proxy::inside::Client;
use diverge_sdk::provider::endpoints::containers::agents::run::server::response::AgenticLoopChunk;
use serde::Serialize;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::OnceCell;

/// The table, made on first use: one row per chunk, by its place.
const CREATE: &str = "CREATE TABLE IF NOT EXISTS human_transcript (id bigint PRIMARY KEY, chunk jsonb NOT NULL)";

/// One chunk of the transcript, by its place in it, from 0.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Entry {
    /// Its place.
    pub id: i64,
    /// The chunk, as the run sent it.
    pub chunk: AgenticLoopChunk,
}

/// The pool, made and the table ensured on first use. A failure leaves
/// it unmade, so the next use tries again.
#[derive(Debug, Default)]
pub struct Store {
    pool: OnceCell<PgPool>,
}

impl Store {
    /// The pool, through the proxy's loopback pgwire.
    async fn pool(&self, client: &Client) -> Result<&PgPool, String> {
        self.pool
            .get_or_try_init(|| async {
                let pool = PgPoolOptions::new().max_connections(2).connect(&client.postgres_url()).await.map_err(|error| error.to_string())?;
                sqlx::query(CREATE).execute(&pool).await.map_err(|error| error.to_string())?;
                Ok::<_, String>(pool)
            })
            .await
    }

    /// The whole transcript, oldest first. A row whose chunk no longer
    /// reads is skipped.
    pub async fn load(&self, client: &Client) -> Result<Vec<Entry>, String> {
        let pool = self.pool(client).await?;
        let rows: Vec<(i64, serde_json::Value)> = sqlx::query_as("SELECT id, chunk FROM human_transcript ORDER BY id").fetch_all(pool).await.map_err(|error| error.to_string())?;
        Ok(rows.into_iter().filter_map(|(id, chunk)| serde_json::from_value(chunk).ok().map(|chunk| Entry { id, chunk })).collect())
    }

    /// One entry kept.
    pub async fn append(&self, client: &Client, entry: &Entry) -> Result<(), String> {
        let pool = self.pool(client).await?;
        let chunk = serde_json::to_value(&entry.chunk).map_err(|error| error.to_string())?;
        sqlx::query("INSERT INTO human_transcript (id, chunk) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET chunk = EXCLUDED.chunk")
            .bind(entry.id)
            .bind(chunk)
            .execute(pool)
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
