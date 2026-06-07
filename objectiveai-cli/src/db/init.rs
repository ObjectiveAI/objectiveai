//! Pool construction + schema bootstrap.
//!
//! No migration framework — schema lives inline as `CREATE TABLE IF
//! NOT EXISTS` etc. that we run on every cold `init`. Idempotent: a
//! second invocation against an already-populated database is a no-op.
//!
//! Connection URL strategy (postmaster is spun up by
//! [`crate::postgres::bootstrap`], with `pg_hba.conf` rewritten to
//! `trust` so we never need a password):
//!
//! - **Unix**: `postgres:///<db>?host=<data_dir>&user=postgres` — talks
//!   to the Unix socket at `<data_dir>/.s.PGSQL.1`; the URL percent-
//!   encodes the data-dir path so non-alphanumeric bytes survive.
//! - **Windows**: `postgres://postgres@127.0.0.1:5432/<db>` — TCP
//!   loopback to the postmaster listening at the canonical port.
//!
//! We open two pools sequentially: a small admin pool against the
//! `postgres` system database (used only to `CREATE DATABASE objectiveai`
//! when it doesn't exist), then the real application pool against the
//! freshly-ensured `objectiveai` database. Schema runs inside one
//! transaction.

use std::path::Path;

use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor as _, Row as _};

use super::{Error, Pool};

/// Database name we create + connect to for the application pool.
const APP_DB_NAME: &str = "objectiveai";

/// Inline schema applied on every cold `init`. `CREATE TABLE IF NOT
/// EXISTS` + `CREATE INDEX IF NOT EXISTS` everywhere keeps the call
/// idempotent — re-running against an existing database is a no-op.
/// No migration framework: nobody is on this DB yet, and adding one
/// would add ceremony we don't need.
const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS messages (
    id                          BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy    TEXT      NOT NULL,
    response_id                 TEXT      NOT NULL,
    kind                        TEXT      NOT NULL,
    path                        TEXT      NOT NULL,
    timestamp                   BIGINT    NOT NULL,
    "index"                     BIGINT    NOT NULL
);
CREATE INDEX IF NOT EXISTS messages_agent_index_idx
    ON messages(agent_instance_hierarchy, "index");
CREATE INDEX IF NOT EXISTS messages_agent_instance_hierarchyx
    ON messages(agent_instance_hierarchy);

CREATE TABLE IF NOT EXISTS messages_queue (
    caller_agent_instance_hierarchy  TEXT   NOT NULL,
    spawned_agent_instance_hierarchy TEXT   NOT NULL,
    "index"                          BIGINT NOT NULL,
    PRIMARY KEY (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
);

CREATE TABLE IF NOT EXISTS files (
    id   BIGSERIAL PRIMARY KEY,
    path TEXT      NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS tags (
    name                            TEXT PRIMARY KEY NOT NULL,
    agent_instance_hierarchy        TEXT,
    parent_agent_instance_hierarchy TEXT,
    agent_full_id                   TEXT,
    updated_at                      BIGINT NOT NULL,
    CHECK (
        (agent_instance_hierarchy IS NOT NULL
         AND parent_agent_instance_hierarchy IS NULL
         AND agent_full_id IS NULL)
        OR
        (agent_instance_hierarchy IS NULL
         AND parent_agent_instance_hierarchy IS NOT NULL
         AND agent_full_id IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS tags_hierarchy_idx
    ON tags(agent_instance_hierarchy);
CREATE INDEX IF NOT EXISTS tags_pending_match_idx
    ON tags(agent_full_id, parent_agent_instance_hierarchy);

CREATE TABLE IF NOT EXISTS prompts (
    id                       BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy TEXT,
    agent_tag                TEXT,
    prompt                   TEXT   NOT NULL,
    enqueued_at              BIGINT NOT NULL,
    key                      TEXT,
    CHECK (
        (agent_instance_hierarchy IS NOT NULL AND agent_tag IS NULL)
        OR
        (agent_instance_hierarchy IS NULL AND agent_tag IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS prompts_hierarchy_idx
    ON prompts(agent_instance_hierarchy, id)
    WHERE agent_instance_hierarchy IS NOT NULL;
CREATE INDEX IF NOT EXISTS prompts_tag_idx
    ON prompts(agent_tag, id)
    WHERE agent_tag IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS prompts_key_hierarchy_unique_idx
    ON prompts(agent_instance_hierarchy, key)
    WHERE agent_instance_hierarchy IS NOT NULL AND key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS prompts_key_tag_unique_idx
    ON prompts(agent_tag, key)
    WHERE agent_tag IS NOT NULL AND key IS NOT NULL;

CREATE TABLE IF NOT EXISTS prompt_contents (
    id        BIGSERIAL PRIMARY KEY,
    prompt_id BIGINT NOT NULL,
    kind      TEXT   NOT NULL
        CHECK (kind IN ('text','image','audio','video','file')),
    FOREIGN KEY (prompt_id) REFERENCES prompts(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS prompt_contents_prompt_idx
    ON prompt_contents(prompt_id);

CREATE TABLE IF NOT EXISTS prompt_texts (
    id   BIGINT PRIMARY KEY,
    text TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_images (
    id     BIGINT PRIMARY KEY,
    url    TEXT   NOT NULL,
    detail TEXT,
    FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_audios (
    id     BIGINT PRIMARY KEY,
    data   TEXT   NOT NULL,
    format TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_videos (
    id  BIGINT PRIMARY KEY,
    url TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS prompt_files (
    id        BIGINT PRIMARY KEY,
    file_data TEXT,
    file_id   TEXT,
    filename  TEXT,
    file_url  TEXT,
    FOREIGN KEY (id) REFERENCES prompt_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS schedules (
    id                       BIGSERIAL PRIMARY KEY,
    name                     TEXT   NOT NULL UNIQUE,
    command                  TEXT   NOT NULL,
    description              TEXT   NOT NULL,
    agent_instance_hierarchy TEXT   NOT NULL,
    interval_seconds         BIGINT
        CHECK (interval_seconds IS NULL OR interval_seconds >= 0),
    agent_arguments          TEXT   NOT NULL,
    created_at               BIGINT NOT NULL,
    last_ran_at              BIGINT
);
"#;

/// Open the admin pool to the `postgres` system database, ensure
/// `objectiveai` exists, then open the application pool and apply the
/// inline schema. Idempotent across cold and warm starts — re-running
/// against an already-bootstrapped database is a no-op (every CREATE
/// uses `IF NOT EXISTS`).
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

    // 2. App pool: connect to the just-ensured database, apply schema
    //    inside one transaction.
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&app_url)
        .await?;
    {
        let mut tx = pool.begin().await?;
        tx.execute(SCHEMA).await?;
        tx.commit().await?;
    }

    Ok(Pool(pool))
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
