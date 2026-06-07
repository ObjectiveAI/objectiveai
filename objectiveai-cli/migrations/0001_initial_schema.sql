-- Initial schema port of the sqlite triple (`db.sqlite` / `tags.sqlite`
-- / `tasks.sqlite`) into one postgres database. Idempotent via
-- `IF NOT EXISTS` on every CREATE; rerunnable as a no-op.
--
-- Naming carries over verbatim from the sqlite originals; the "index"
-- column is reserved in postgres and stays double-quoted everywhere.

-- ---------------------------------------------------------------------------
-- messages — one row per request / response / notification. The base
-- per-stream log of what every agent has produced. Two indexes:
-- `(agent_instance_hierarchy, "index")` for the watermark-bound reads,
-- and `(agent_instance_hierarchy)` for the prefix-LIKE scans in
-- `list_direct_active_children`.
-- ---------------------------------------------------------------------------

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

-- ---------------------------------------------------------------------------
-- messages_queue — per-(caller, spawned) watermark of the highest
-- `messages."index"` the caller has already consumed. Composite PK
-- doubles as the lookup index.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS messages_queue (
    caller_agent_instance_hierarchy  TEXT   NOT NULL,
    spawned_agent_instance_hierarchy TEXT   NOT NULL,
    "index"                          BIGINT NOT NULL,
    PRIMARY KEY (caller_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
);

-- ---------------------------------------------------------------------------
-- files — lazy id↔path table populated on demand by
-- `read_new_from_queue`. `UNIQUE(path)` keeps one id per path forever
-- so `agents read id <id>` resolves deterministically across processes.
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS files (
    id   BIGSERIAL PRIMARY KEY,
    path TEXT      NOT NULL UNIQUE
);

-- ---------------------------------------------------------------------------
-- tags — client-side agent tags. Each row is in exactly one of two
-- states (BOUND vs PENDING) per the CHECK constraint. Re-tagging hits
-- `INSERT … ON CONFLICT (name) DO UPDATE SET …` (the postgres analog
-- of sqlite's `INSERT OR REPLACE`).
-- ---------------------------------------------------------------------------

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

-- ---------------------------------------------------------------------------
-- prompts — deferred-message queue rows from
-- `agents message-queue add` and the
-- `agents instances message` enqueue path. Each row targets either an
-- `agent_instance_hierarchy` OR an `agent_tag` (CHECK enforces).
-- ---------------------------------------------------------------------------

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
-- Partial unique indexes scope `key` uniqueness per target. A second
-- `agents message-queue add` with the same (target, key) is upserted
-- by the storage helper (explicit DELETE + INSERT). NULL keys are
-- excluded from the index, so unkeyed items remain append-only.
CREATE UNIQUE INDEX IF NOT EXISTS prompts_key_hierarchy_unique_idx
    ON prompts(agent_instance_hierarchy, key)
    WHERE agent_instance_hierarchy IS NOT NULL AND key IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS prompts_key_tag_unique_idx
    ON prompts(agent_tag, key)
    WHERE agent_tag IS NOT NULL AND key IS NOT NULL;

-- ---------------------------------------------------------------------------
-- prompt_contents + per-kind tables — master content registry, FK-
-- anchored at `prompt_id` so a single `DELETE FROM prompts WHERE id =
-- ?` cascades the entire prompt out. Per-kind tables share the
-- master's row id 1:1 and cascade again on the per-kind FK.
-- ---------------------------------------------------------------------------

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

-- ---------------------------------------------------------------------------
-- schedules — `agents tasks schedule` storage. `interval_seconds`
-- nullable: NULL = oneshot (runner fires once + deletes); non-NULL =
-- recurring with that minimum interval. `last_ran_at` starts NULL on
-- insert and is bumped by the runner on each fire. `name` is globally
-- UNIQUE so `agents tasks run` tagging is unambiguous.
-- ---------------------------------------------------------------------------

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
