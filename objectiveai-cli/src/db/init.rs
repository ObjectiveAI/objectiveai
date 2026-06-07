//! Pool construction + migration runner.
//!
//! Connection URL strategy (postmaster is spun up by
//! [`crate::postgres::bootstrap`], with `pg_hba.conf` rewritten to
//! `trust` so we never need a password):
//!
//! - **Unix**: `postgres:///<db>?host=<data_dir>&user=postgres` — talks
//!   to the Unix socket at `<data_dir>/.s.PGSQL.1`; the URL must
//!   percent-encode the data-dir path so non-alphanumeric bytes survive.
//! - **Windows**: `postgres://postgres@127.0.0.1:5432/<db>` — TCP
//!   loopback to the postmaster listening at the canonical port.
//!
//! We open two pools sequentially: a small admin pool against the
//! `postgres` system database (used only to `CREATE DATABASE objectiveai`
//! when it doesn't exist), then the real application pool against the
//! freshly-ensured `objectiveai` database. Migrations are baked in via
//! `include_str!` so the binary doesn't need the source tree at run
//! time, and run inside one transaction to keep failure all-or-nothing.

use std::path::Path;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor as _, Row as _};

use super::{Error, Pool};

/// Embedded migration SQL — applied unconditionally on every cold
/// `init`. `CREATE TABLE IF NOT EXISTS` everywhere keeps the call
/// idempotent.
const MIGRATION_INITIAL: &str = include_str!("../../migrations/0001_initial_schema.sql");

/// Database name we create + connect to for the application pool.
const APP_DB_NAME: &str = "objectiveai";

/// Open the admin pool to the `postgres` system database, ensure
/// `objectiveai` exists, then open the application pool and run all
/// migrations.
pub async fn init(config_base_dir: &Path) -> Result<Pool, Error> {
    let data_dir = config_base_dir.join("db");
    let admin_url = build_url(&data_dir, "postgres");
    let app_url = build_url(&data_dir, APP_DB_NAME);

    // 1. Admin pool: ensure the `objectiveai` database exists.
    //    `CREATE DATABASE` cannot run inside a transaction, so we use
    //    a fresh single-connection pool just for this check + insert.
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(&admin_url)
        .await?;
    let exists: bool = {
        let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(APP_DB_NAME)
            .fetch_one(&admin)
            .await?;
        row.try_get::<bool, _>(0)?
    };
    if !exists {
        // `CREATE DATABASE` can't be parameterised; the constant is
        // a compile-time literal so no injection surface.
        admin
            .execute(format!("CREATE DATABASE {APP_DB_NAME}").as_str())
            .await?;
    }
    admin.close().await;

    // 2. App pool: connect to the just-ensured database, apply
    //    migrations.
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&app_url)
        .await?;
    run_migrations(&pool).await?;

    Ok(Pool(pool))
}

/// Apply every embedded migration in a single transaction. Each
/// migration is its own `\n;\n`-terminated SQL string today (we only
/// ship one — `0001_initial_schema.sql` — but the loop scales as more
/// land). All migrations use `CREATE TABLE IF NOT EXISTS` and
/// `CREATE INDEX IF NOT EXISTS` so reruns are a no-op.
async fn run_migrations(pool: &sqlx::PgPool) -> Result<(), Error> {
    let mut tx = pool.begin().await?;
    tx.execute(MIGRATION_INITIAL).await?;
    tx.commit().await?;
    Ok(())
}

/// Compose the libpq connection URL for `db` against the postmaster
/// rooted at `data_dir`. On Unix the host is the data-dir path (which
/// holds the `.s.PGSQL.1` socket); on Windows we go through TCP
/// loopback at 5432.
fn build_url(data_dir: &Path, db: &str) -> String {
    #[cfg(unix)]
    {
        let host_param = percent_encoding::utf8_percent_encode(
            &data_dir.to_string_lossy(),
            percent_encoding::NON_ALPHANUMERIC,
        );
        format!("postgres:///{db}?host={host_param}&user=postgres")
    }
    #[cfg(windows)]
    {
        let _ = data_dir;
        format!("postgres://postgres@127.0.0.1:5432/{db}")
    }
}
