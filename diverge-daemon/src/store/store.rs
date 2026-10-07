//! The pool, opened once, and the transactions begun on it.

use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use sqlx::{Connection as _, PgConnection, Postgres, Transaction};

use super::{Error, root, schema};

/// The database the daemon's own records are in, when the daemon
/// runs its own cluster. A remote URL names its database itself.
const DATABASE: &str = "diverge";

/// How many connections the pool holds at most.
const CONNECTIONS: u32 = 8;

/// The records, reached through a pool.
#[derive(Debug, Clone)]
pub struct Store {
    /// The pool.
    pool: PgPool,
}

/// Open the store at `url`, apply the schema, and seed root into a
/// fresh database.
///
/// `local` is the daemon's own cluster, whose URL names no database:
/// `diverge` is made in it if absent — the daemon is its superuser —
/// and opened. A remote URL is opened exactly as given; the operator
/// names the database and it exists already. Then, in one
/// transaction, the schema is applied and, when the database had no
/// `accounts` table before, the root role and account are seeded.
pub async fn open(url: &str, local: bool) -> Result<Store, Error> {
    let options: PgConnectOptions = url.parse()?;
    let options = if local {
        create_database(options.clone()).await?;
        options.database(DATABASE)
    } else {
        options
    };
    let pool = PgPoolOptions::new().max_connections(CONNECTIONS).connect_with(options).await?;
    let store = Store { pool };
    let mut tx = store.begin().await?;
    if schema::apply(&mut tx).await? {
        root::seed(&mut tx).await?;
    }
    tx.commit().await?;
    Ok(store)
}

/// Make the daemon's database in a local cluster if it is absent, on a
/// connection to the cluster's own `postgres` database, since
/// `CREATE DATABASE` cannot run inside a transaction and the pool
/// has to be opened on the database it will use.
async fn create_database(options: PgConnectOptions) -> Result<(), Error> {
    let mut conn = PgConnection::connect_with(&options.database("postgres")).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(DATABASE)
        .fetch_one(&mut conn)
        .await?;
    if !exists {
        // The name is this crate's own constant, so it is quoted here
        // rather than bound: DDL takes no parameters.
        sqlx::query(&format!("CREATE DATABASE \"{DATABASE}\"")).execute(&mut conn).await?;
    }
    conn.close().await?;
    Ok(())
}

impl Store {
    /// Begin a transaction. Every request that writes runs inside
    /// one, and commits it only on the answer that says it was done;
    /// dropped, it is the rollback.
    pub async fn begin(&self) -> Result<Transaction<'static, Postgres>, Error> {
        Ok(self.pool.begin().await?)
    }

    /// One connection, for a read that needs no transaction.
    pub async fn acquire(&self) -> Result<PoolConnection<Postgres>, Error> {
        Ok(self.pool.acquire().await?)
    }
}
