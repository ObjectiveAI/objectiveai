//! Pool construction + schema bootstrap.
//!
//! No migration framework — schema lives inline as `CREATE TABLE IF
//! NOT EXISTS` etc. that we run on every cold `init`. Idempotent: a
//! second invocation against an already-populated database is a no-op.
//!
//! Connection URL strategy: the caller (`Context::db_client()`)
//! hands [`init`] a base connect URL WITHOUT a database path — either
//! the `postgresql://...` published in the `db` spawn lock, or one
//! composed from `config db` via [`config_url`] when `db.address`
//! points at a remote postgres — plus the application database name
//! (`config db.database`, default `objectiveai`).
//!
//! We open two pools sequentially: a small admin pool against the
//! base URL as-is (no path segment, so it lands on the connecting
//! user's default database, `postgres`), used only to
//! `CREATE DATABASE` the application database when it doesn't exist;
//! then the real application pool against the freshly-ensured
//! database. Schema runs inside one transaction.

use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor as _, Row as _};

use super::{Error, Pool};

/// The base `objectiveai.*` schema, applied on every cold `init`.
/// Pulled from `src/db/schema.sql` so the canonical definitions live
/// in a real .sql file (tooling-readable, editor-highlighted) beside
/// the `logs`/`channels` schemas — not a string baked into Rust.
/// `CREATE ... IF NOT EXISTS` everywhere keeps the call idempotent;
/// no migration framework (nobody is on this DB yet).
const SCHEMA: &str = include_str!("schema.sql");

/// `logs.*` schema. Pulled from `src/db/logs/schema.sql` so the
/// canonical definitions live in a real .sql file (readable by
/// tooling, syntax-highlighted by editors) instead of as a string
/// constant baked into Rust source.
const LOGS_SCHEMA: &str = include_str!("logs/schema.sql");

/// `channels.*` schema (durable duplex channels + message log). Pulled
/// from `src/db/channels/schema.sql`, applied alongside [`LOGS_SCHEMA`].
const CHANNELS_SCHEMA: &str = include_str!("channels/schema.sql");

/// The shared readonly group every plugin/tool compartment role
/// joins (see [`super::compartment`]): USAGE + SELECT over the base
/// `objectiveai` schema, with default privileges so tables the base
/// schema grows LATER are covered automatically. Applied inside the
/// same advisory-locked schema-apply step as the table DDL;
/// `CREATE ROLE` has no `IF NOT EXISTS`, hence the DO block (roles
/// are cluster-wide — a sibling database in the same cluster may
/// have created it already; the GRANTs are per-database and re-run
/// for each).
const READER_GROUP: &str = r#"
DO $$
BEGIN
    CREATE ROLE objectiveai_read NOLOGIN;
EXCEPTION WHEN duplicate_object THEN
    NULL;
END
$$;
GRANT USAGE ON SCHEMA objectiveai TO objectiveai_read;
GRANT SELECT ON ALL TABLES IN SCHEMA objectiveai TO objectiveai_read;
ALTER DEFAULT PRIVILEGES IN SCHEMA objectiveai GRANT SELECT ON TABLES TO objectiveai_read;
"#;

/// Open the admin pool against `url` (no database path — lands on the
/// connecting user's default database, `postgres`), ensure `database`
/// exists, then open the application pool and apply the inline
/// schema. Idempotent across cold and warm starts — re-running
/// against an already-bootstrapped database is a no-op (every CREATE
/// uses `IF NOT EXISTS`).
pub async fn init(url: &str, database: &str) -> Result<Pool, Error> {
    let app_url = format!(
        "{}/{}",
        url.trim_end_matches('/'),
        percent_encoding::utf8_percent_encode(database, percent_encoding::NON_ALPHANUMERIC),
    );

    // 1. Admin pool: ensure the application database exists.
    //    `CREATE DATABASE` cannot run inside a transaction, so we use
    //    a fresh single-connection pool just for this check + insert.
    let admin = PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .map_err(|e| unreachable_hint(url, e))?;
    let exists: bool = {
        let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(database)
            .fetch_one(&admin)
            .await?;
        row.try_get::<bool, _>(0)?
    };
    if !exists {
        // `CREATE DATABASE` can't be parameterised; the name comes
        // from config, so quote it as an identifier (double-quote
        // wrapping with embedded quotes doubled) to keep arbitrary
        // names safe.
        //
        // Race: two concurrent cli processes can both observe
        // `exists = false` and race the CREATE. The second to
        // commit gets SQLSTATE 42P04 (`duplicate_database`); swallow
        // that exact code (any other error still propagates).
        let quoted = database.replace('"', "\"\"");
        match admin
            .execute(format!("CREATE DATABASE \"{quoted}\"").as_str())
            .await
        {
            Ok(_) => {}
            // 42P04 = `duplicate_database` (the high-level check
            // postgres fires when it sees an existing matching
            // datname row in pg_database).
            // 23505 = `unique_violation` on `pg_database_datname_index`
            // (the low-level catalog insert losing the race).
            // Either way, the database exists now; continue.
            Err(sqlx::Error::Database(db))
                if matches!(db.code().as_deref(), Some("42P04") | Some("23505")) => {}
            Err(e) => return Err(e.into()),
        }
    }
    admin.close().await;

    // 2. App pool: connect to the just-ensured database, apply schema
    //    inside one transaction.
    //
    //    Concurrency: `CREATE … IF NOT EXISTS` is not atomic across
    //    parallel sessions (two can both see "doesn't exist" and
    //    both try to insert into `pg_class`; the loser gets 23505
    //    on `pg_class_relname_nsp_index` or a deadlock against
    //    another session writing the same catalog row).
    //    Serialize the schema-apply step behind a session-level
    //    advisory lock so only one process at a time runs it; the
    //    `IF NOT EXISTS` clauses then make every subsequent run a
    //    no-op.
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&app_url)
        .await
        .map_err(|e| unreachable_hint(&app_url, e))?;
    {
        // Arbitrary 64-bit constant; the pair `(database, key)`
        // defines the lock identity, so as long as every process
        // uses the same key against the same db they serialize on
        // schema-apply.
        const SCHEMA_LOCK_KEY: i64 = 0x0B7EC71AE_15CBE_AA_i64;
        let mut conn = pool.acquire().await?;
        sqlx::query("SELECT pg_advisory_lock($1)")
            .bind(SCHEMA_LOCK_KEY)
            .execute(&mut *conn)
            .await?;
        let apply_result: Result<(), Error> = async {
            conn.execute(SCHEMA).await?;
            conn.execute(LOGS_SCHEMA).await?;
            conn.execute(CHANNELS_SCHEMA).await?;
            conn.execute(READER_GROUP).await?;
            Ok(())
        }
        .await;
        // Best-effort release; if the connection died the lock
        // releases on session end anyway.
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(SCHEMA_LOCK_KEY)
            .execute(&mut *conn)
            .await;
        apply_result?;
    }

    Ok(Pool(pool))
}

/// Compose the base connect URL (no database path) for a remote
/// postgres configured via `config db` — used by `Context::db_client`
/// when `db.address` is set, in place of the spawn lock's URL. The
/// address may embed a port (`host:5432`); a bare host gets postgres's
/// default port. User and password are percent-encoded so arbitrary
/// config values can't break the URL shape.
pub(crate) fn config_url(address: &str, user: &str, password: &str) -> String {
    let user =
        percent_encoding::utf8_percent_encode(user, percent_encoding::NON_ALPHANUMERIC);
    let password =
        percent_encoding::utf8_percent_encode(password, percent_encoding::NON_ALPHANUMERIC);
    format!("postgres://{user}:{password}@{address}")
}

/// Map a pool-connect failure to the actionable error: the database
/// at the resolved URL isn't reachable — the daemon spawns the local
/// objectiveai-db on demand, so the fix is a retry or `db config
/// address` (remote postgres). Non-connect errors (auth failures,
/// TLS, ...) get the same wrapper — the remedy hint is still the
/// right one.
fn unreachable_hint(url: &str, source: sqlx::Error) -> Error {
    Error::DbUnreachable {
        url: redact_url_password(url),
        source: Box::new(source),
    }
}

/// `scheme://user:password@rest` → `scheme://user:***@rest`, so the
/// connect URL can ride in an error message without leaking the
/// password. Returned unchanged when there is no userinfo or no
/// password segment.
fn redact_url_password(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let rest = &url[scheme_end + 3..];
    let authority = &rest[..rest.find('/').unwrap_or(rest.len())];
    let Some(at) = authority.rfind('@') else {
        return url.to_string();
    };
    let userinfo = &authority[..at];
    let Some(colon) = userinfo.find(':') else {
        return url.to_string();
    };
    format!(
        "{}:***{}",
        &url[..scheme_end + 3 + colon],
        &url[scheme_end + 3 + at..],
    )
}

#[cfg(test)]
mod tests {
    use super::redact_url_password;

    #[test]
    fn redact_url_password_cases() {
        assert_eq!(
            redact_url_password("postgresql://postgres:s3cr%40t@127.0.0.1:5432"),
            "postgresql://postgres:***@127.0.0.1:5432",
        );
        assert_eq!(
            redact_url_password("postgres://u:p@h:5432/objectiveai"),
            "postgres://u:***@h:5432/objectiveai",
        );
        // No password segment in the userinfo.
        assert_eq!(
            redact_url_password("postgres://postgres@127.0.0.1:5432"),
            "postgres://postgres@127.0.0.1:5432",
        );
        // No userinfo at all.
        assert_eq!(
            redact_url_password("postgres://127.0.0.1:5432/db"),
            "postgres://127.0.0.1:5432/db",
        );
        // Not even a scheme.
        assert_eq!(redact_url_password("127.0.0.1:5432"), "127.0.0.1:5432");
    }
}
