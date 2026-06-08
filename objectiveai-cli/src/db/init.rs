//! Pool construction + schema bootstrap.
//!
//! No migration framework — schema lives inline as `CREATE TABLE IF
//! NOT EXISTS` etc. that we run on every cold `init`. Idempotent: a
//! second invocation against an already-populated database is a no-op.
//!
//! Connection URL strategy (postmaster is spun up by
//! [`crate::postgres::bootstrap`], with the cluster's default
//! `pg_hba.conf` left intact; we authenticate with the fixed
//! [`crate::postgres::PG_INITDB_PASSWORD`] that the cluster was
//! initdb'd against):
//!
//! - **Unix**: `postgres:///<db>?host=<data_dir>&user=postgres&password=<pw>`
//!   — talks to the Unix socket at `<data_dir>/.s.PGSQL.1`; the URL
//!   percent-encodes both the data-dir path and the password so non-
//!   alphanumeric bytes survive.
//! - **Windows**: `postgres://postgres:<pw>@127.0.0.1:5432/<db>` — TCP
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
-- `tag_groups`: explicit grouping container that lets many tags
-- share one resolved `AgentSpec` + parent lineage. The cli's
-- `agents tags apply` either creates a group on the fly (for the
-- `--agent` arm) or joins the new tag into an existing group (for
-- the `--agent-tag` arm). When any one tag in a group is picked
-- up by a live spawn, the conduit-driven upgrade flips every tag
-- in the group from `tag_group` to `agent_instance_hierarchy` in
-- one UPDATE inside the read transaction — see
-- `db::message_queue::read_pending_and_upgrade_tag`.
CREATE TABLE IF NOT EXISTS tag_groups (
    id                              BIGSERIAL PRIMARY KEY,
    -- Resolved `agents::spawn::AgentSpec`; serialized
    -- as JSONB. Favorites are resolved at apply-time, never at
    -- spawn-time, so this column is always inline-or-remote.
    agent_spec                      JSONB  NOT NULL,
    -- The lineage prefix the spawn-by-tag will compose its AIH
    -- against. NOT NULL: callers (CLI handlers) substitute the
    -- cli's own `Config.agent_instance_hierarchy` when the user
    -- omits the argument.
    parent_agent_instance_hierarchy TEXT   NOT NULL,
    created_at                      BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    name                     TEXT PRIMARY KEY NOT NULL,
    -- BOUND when set: this tag resolves to a live AIH.
    agent_instance_hierarchy TEXT,
    -- GROUPED when set: this tag's resolution is the tag_group
    -- row's (agent_spec, parent). Exactly one of the two columns
    -- is non-null at any time.
    tag_group                BIGINT,
    updated_at               BIGINT NOT NULL,
    CHECK (
        (agent_instance_hierarchy IS NOT NULL AND tag_group IS NULL)
        OR
        (agent_instance_hierarchy IS NULL AND tag_group IS NOT NULL)
    ),
    FOREIGN KEY (tag_group) REFERENCES tag_groups(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS tags_hierarchy_idx
    ON tags(agent_instance_hierarchy);
CREATE INDEX IF NOT EXISTS tags_tag_group_idx
    ON tags(tag_group);

-- Latest continuation token per agent_instance_hierarchy. Upserted
-- per streamed chunk by the chunk-yielder loops in `agents spawn`
-- and `functions execute`. No GC, no history — querying it gives
-- the single most recent continuation for that AIH.
CREATE TABLE IF NOT EXISTS agent_continuations (
    agent_instance_hierarchy TEXT PRIMARY KEY NOT NULL,
    continuation             TEXT             NOT NULL,
    updated_at               BIGINT           NOT NULL
);

CREATE TABLE IF NOT EXISTS message_queue (
    id                       BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy TEXT,
    agent_tag                TEXT,
    content                  TEXT   NOT NULL,
    enqueued_at              BIGINT NOT NULL,
    key                      TEXT,
    CHECK (
        (agent_instance_hierarchy IS NOT NULL AND agent_tag IS NULL)
        OR
        (agent_instance_hierarchy IS NULL AND agent_tag IS NOT NULL)
    )
);
CREATE INDEX IF NOT EXISTS message_queue_hierarchy_idx
    ON message_queue(agent_instance_hierarchy, id)
    WHERE agent_instance_hierarchy IS NOT NULL;
CREATE INDEX IF NOT EXISTS message_queue_tag_idx
    ON message_queue(agent_tag, id)
    WHERE agent_tag IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_hierarchy_unique_idx
    ON message_queue(agent_instance_hierarchy, key)
    WHERE agent_instance_hierarchy IS NOT NULL AND key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_tag_unique_idx
    ON message_queue(agent_tag, key)
    WHERE agent_tag IS NOT NULL AND key IS NOT NULL;

CREATE TABLE IF NOT EXISTS message_queue_contents (
    id               BIGSERIAL PRIMARY KEY,
    message_queue_id BIGINT NOT NULL,
    kind             TEXT   NOT NULL
        CHECK (kind IN ('text','image','audio','video','file')),
    FOREIGN KEY (message_queue_id) REFERENCES message_queue(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS message_queue_contents_parent_idx
    ON message_queue_contents(message_queue_id);

CREATE TABLE IF NOT EXISTS message_queue_texts (
    id   BIGINT PRIMARY KEY,
    text TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_queue_images (
    id     BIGINT PRIMARY KEY,
    url    TEXT   NOT NULL,
    detail TEXT,
    FOREIGN KEY (id) REFERENCES message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_queue_audios (
    id     BIGINT PRIMARY KEY,
    data   TEXT   NOT NULL,
    format TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_queue_videos (
    id  BIGINT PRIMARY KEY,
    url TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS message_queue_files (
    id        BIGINT PRIMARY KEY,
    file_data TEXT,
    file_id   TEXT,
    filename  TEXT,
    file_url  TEXT,
    FOREIGN KEY (id) REFERENCES message_queue_contents(id) ON DELETE CASCADE
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

-- AFTER-DELETE trigger on `message_queue`: every clear emits a
-- `NOTIFY message_queue_delete '<id>'` so the cli's
-- `db::message_queue::subscribe_delivered` listener can wake up
-- the instant the conduit's `clear_by_ids` removes our row. No
-- polling — pure native LISTEN/NOTIFY.
CREATE OR REPLACE FUNCTION notify_message_queue_delete()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('message_queue_delete', OLD.id::text);
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS message_queue_delete_notify ON message_queue;
CREATE TRIGGER message_queue_delete_notify
AFTER DELETE ON message_queue
FOR EACH ROW EXECUTE FUNCTION notify_message_queue_delete();

"#;

/// `logs.*` schema. Pulled from `src/db/logs/schema.sql` so the
/// canonical definitions live in a real .sql file (readable by
/// tooling, syntax-highlighted by editors) instead of as a string
/// constant baked into Rust source.
const LOGS_SCHEMA: &str = include_str!("logs/schema.sql");

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
        tx.execute(LOGS_SCHEMA).await?;
        tx.commit().await?;
    }

    Ok(Pool(pool))
}

/// Compose the libpq connection URL for `db` against the postmaster
/// rooted at `data_dir`. On Unix the host is the data-dir path (which
/// holds the `.s.PGSQL.1` socket); on Windows we go through TCP
/// loopback at 5432. The password is the fixed
/// [`crate::postgres::PG_INITDB_PASSWORD`] the cluster was initdb'd
/// against.
fn build_url(data_dir: &Path, db: &str) -> String {
    let password = percent_encoding::utf8_percent_encode(
        crate::postgres::PG_INITDB_PASSWORD,
        percent_encoding::NON_ALPHANUMERIC,
    );
    #[cfg(unix)]
    {
        let host_param = percent_encoding::utf8_percent_encode(
            &data_dir.to_string_lossy(),
            percent_encoding::NON_ALPHANUMERIC,
        );
        format!("postgres:///{db}?host={host_param}&user=postgres&password={password}")
    }
    #[cfg(windows)]
    {
        let _ = data_dir;
        format!("postgres://postgres:{password}@127.0.0.1:5432/{db}")
    }
}
