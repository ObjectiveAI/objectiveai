//! Pool construction + schema bootstrap.
//!
//! No migration framework — schema lives inline as `CREATE TABLE IF
//! NOT EXISTS` etc. that we run on every cold `init`. Idempotent: a
//! second invocation against an already-populated database is a no-op.
//!
//! Connection URL strategy (postmaster is spun up by
//! [`crate::postgres::bootstrap`] before this runs, with the
//! cluster's default `pg_hba.conf` left intact; we authenticate
//! with the fixed [`crate::postgres::PG_INITDB_PASSWORD`] the
//! cluster was initdb'd against):
//!
//! `postgres://postgres:<pw>@127.0.0.1:<port>/<db>` — TCP
//! loopback on every platform. `<port>` is the OS-assigned
//! ephemeral port the postmaster bound to at bootstrap time;
//! it's read out of `<data_dir>/postmaster.pid` (line 4) via
//! [`crate::postgres::pg_port`].
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
    id                              BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy        TEXT,
    agent_tag                       TEXT,
    -- AIH of the caller who enqueued this row (sourced from
    -- `ctx.config.agent_instance_hierarchy` at enqueue time).
    -- Surfaced on `agents queue read pending` so callers can
    -- audit "who asked for this" without a join.
    sender_agent_instance_hierarchy TEXT   NOT NULL,
    -- Content rows live in `message_queue_contents` (PK
    -- `id BIGSERIAL`, FK `message_queue_id` → here). Readers
    -- JOIN; no denormalized JSON shadow column lives here.
    enqueued_at                     BIGINT NOT NULL,
    key                             TEXT,
    -- Soft-delete marker. Rows start at TRUE and flip to FALSE
    -- when consumed (either via the LogWriter's MessageQueue row
    -- write or via `db::message_queue::delete_by_id`'s in-flight
    -- lock-race-released path). Every reader filters
    -- `WHERE active = TRUE`, so flipped rows are invisible.
    -- Content stays around in `message_queue_contents` (the old
    -- `ON DELETE CASCADE` chain no longer fires because we don't
    -- DELETE).
    active                          BOOLEAN NOT NULL DEFAULT TRUE,
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
-- Per-target idempotency keys. The `AND active = TRUE` clause
-- means inactive prior rows don't count toward uniqueness — an
-- `agents message --enqueue-with-key k` after a prior consumption
-- inserts cleanly without UNIQUE-violating the soft-flipped row.
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_hierarchy_unique_idx
    ON message_queue(agent_instance_hierarchy, key)
    WHERE agent_instance_hierarchy IS NOT NULL
      AND key IS NOT NULL
      AND active = TRUE;
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_tag_unique_idx
    ON message_queue(agent_tag, key)
    WHERE agent_tag IS NOT NULL
      AND key IS NOT NULL
      AND active = TRUE;

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
    name                     TEXT   NOT NULL,
    command                  TEXT   NOT NULL,
    description              TEXT   NOT NULL,
    agent_instance_hierarchy TEXT   NOT NULL,
    interval_seconds         BIGINT
        CHECK (interval_seconds IS NULL OR interval_seconds >= 0),
    agent_arguments          TEXT   NOT NULL,
    -- Versions are separate rows: `tasks schedule --overwrite` INSERTS
    -- a new row with version = max+1 that shadows the older versions
    -- of the same (name, aih). Shadowed rows never run again and never
    -- list, but stay on disk so `tasks_runs` history is per-version.
    -- Rows are NEVER deleted.
    version                  BIGINT NOT NULL DEFAULT 1,
    -- Plugin coordinate of the plugin that registered this schedule,
    -- captured from `ctx.plugin` at schedule time. All three NULL when
    -- not scheduled by a plugin; all three set when it was (enforced by
    -- the CHECK below).
    plugin_owner             TEXT,
    plugin_repository        TEXT,
    plugin_version           TEXT,
    created_at               BIGINT NOT NULL,
    -- A schedule name is unique per agent instance hierarchy and
    -- version — two different agents may reuse the same name, and one
    -- agent's schedule keeps every version it ever had as rows.
    UNIQUE (name, agent_instance_hierarchy, version),
    CHECK (
        (plugin_owner IS NULL AND plugin_repository IS NULL AND plugin_version IS NULL)
        OR
        (plugin_owner IS NOT NULL AND plugin_repository IS NOT NULL AND plugin_version IS NOT NULL)
    )
);

-- One row per schedule firing. A oneshot is exhausted the moment it
-- has a run; a recurring row's readiness keys off its newest run.
-- Written atomically by the same claim query that selects the tasks
-- `tasks run` fires, so concurrent runners never double-claim.
CREATE TABLE IF NOT EXISTS tasks_runs (
    id          BIGSERIAL PRIMARY KEY,
    schedule_id BIGINT NOT NULL REFERENCES schedules(id),
    ran_at      BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS tasks_runs_schedule_idx
    ON tasks_runs(schedule_id, ran_at DESC);

-- One row per item a fired task emitted on `tasks run`'s stream,
-- serialized exactly as the wire envelope, linked to its run.
CREATE TABLE IF NOT EXISTS tasks_logs (
    id         BIGSERIAL PRIMARY KEY,
    run_id     BIGINT NOT NULL REFERENCES tasks_runs(id),
    value      TEXT   NOT NULL,
    created_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS tasks_logs_run_idx
    ON tasks_logs(run_id, id);

-- AFTER-UPDATE trigger on `message_queue.active`: every soft-flip
-- (TRUE → FALSE) emits a `NOTIFY message_queue_inactive '<id>'`
-- so the cli's `db::message_queue::subscribe_delivered` listener
-- wakes up the instant a consumption flip lands. Pure native
-- LISTEN/NOTIFY — no polling. We no longer hard-delete, so the
-- prior AFTER DELETE trigger is gone.
CREATE OR REPLACE FUNCTION notify_message_queue_inactive()
RETURNS trigger AS $$
BEGIN
    IF OLD.active = TRUE AND NEW.active = FALSE THEN
        PERFORM pg_notify('message_queue_inactive', NEW.id::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER message_queue_inactive_notify
AFTER UPDATE OF active ON message_queue
FOR EACH ROW EXECUTE FUNCTION notify_message_queue_inactive();

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
    // `postgres::bootstrap` ran first inside `Context::new`, so
    // `postmaster.pid` is in place and carries the port the
    // postmaster bound to (OS-assigned at start time).
    let port = crate::postgres::pg_port(&data_dir).await.ok_or_else(|| {
        Error::InvalidData(format!(
            "postmaster.pid in {data_dir:?} did not carry a parseable port — \
             bootstrap appears to have run but the postmaster isn't running"
        ))
    })?;
    let admin_url = build_url("postgres", port);
    let app_url = build_url(APP_DB_NAME, port);

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
        //
        // Race: two concurrent cli processes can both observe
        // `exists = false` and race the CREATE. The second to
        // commit gets SQLSTATE 42P04 (`duplicate_database`); swallow
        // that exact code (any other error still propagates).
        match admin
            .execute(format!("CREATE DATABASE {APP_DB_NAME}").as_str())
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
        .await?;
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

/// Compose the libpq connection URL for `db` against the
/// postmaster listening on `127.0.0.1:<port>`. TCP loopback on
/// every platform; the port is the OS-assigned ephemeral one
/// the postmaster bound to at bootstrap time and advertised via
/// `postmaster.pid`. Password is the fixed
/// [`crate::postgres::PG_INITDB_PASSWORD`] the cluster was
/// initdb'd against.
fn build_url(db: &str, port: u16) -> String {
    let password = percent_encoding::utf8_percent_encode(
        crate::postgres::PG_INITDB_PASSWORD,
        percent_encoding::NON_ALPHANUMERIC,
    );
    format!("postgres://postgres:{password}@127.0.0.1:{port}/{db}")
}
