-- Every base objectiveai table lives in the `objectiveai` schema —
-- one namespace for what used to be split across `public` and
-- `logs`. Plugin/tool compartments (see `db::compartment`) get
-- readonly over exactly this schema and never touch `public`.
CREATE SCHEMA IF NOT EXISTS objectiveai;

-- `tag_groups`: explicit grouping container that lets many tags
-- share one resolved `AgentSpec` + parent lineage. The cli's
-- `agents tags apply` either creates a group on the fly (for the
-- `--agent` arm) or joins the new tag into an existing group (for
-- the `--agent-tag` arm). When any one tag in a group is picked
-- up by a live spawn, the conduit-driven upgrade flips every tag
-- in the group from `tag_group` to `agent_instance_hierarchy` in
-- one UPDATE inside the read transaction — see
-- `db::message_queue::read_pending_and_upgrade_tag`.
CREATE TABLE IF NOT EXISTS objectiveai.tag_groups (
    id                              BIGSERIAL PRIMARY KEY,
    -- Resolved `agents::spawn::AgentSpec`; serialized
    -- as JSONB. References are resolved at apply-time, never at
    -- spawn-time, so this column is always inline-or-remote.
    agent_spec                      JSONB  NOT NULL,
    -- The lineage prefix the spawn-by-tag will compose its AIH
    -- against. NOT NULL: callers (CLI handlers) substitute the
    -- cli's own `Config.agent_instance_hierarchy` when the user
    -- omits the argument.
    parent_agent_instance_hierarchy TEXT   NOT NULL,
    created_at                      BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS objectiveai.tags (
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
    FOREIGN KEY (tag_group) REFERENCES objectiveai.tag_groups(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS tags_hierarchy_idx
    ON objectiveai.tags(agent_instance_hierarchy);
CREATE INDEX IF NOT EXISTS tags_tag_group_idx
    ON objectiveai.tags(tag_group);

-- Fires `NOTIFY tags_changed '<agent_instance_hierarchy>'` whenever a
-- tag row bound to an AIH is written or removed, so the daemon's
-- `/agents/instances/list` endpoint can refresh that agent's tag list. Only BOUND
-- rows carry an AIH; GROUPED rows (`agent_instance_hierarchy` NULL) map
-- to no `/agents/instances/list` record and are skipped. A relocation (an UPDATE that
-- moves the AIH) changes BOTH the old and new agent's tag list, so both
-- sides are notified when they differ.
CREATE OR REPLACE FUNCTION objectiveai.notify_tags_changed()
RETURNS trigger AS $$
BEGIN
    IF (TG_OP = 'DELETE') THEN
        IF OLD.agent_instance_hierarchy IS NOT NULL THEN
            PERFORM pg_notify('tags_changed', OLD.agent_instance_hierarchy);
        END IF;
        RETURN OLD;
    END IF;
    -- INSERT / UPDATE. On an AIH move, notify the vacated side too.
    IF TG_OP = 'UPDATE'
       AND OLD.agent_instance_hierarchy IS NOT NULL
       AND OLD.agent_instance_hierarchy IS DISTINCT FROM NEW.agent_instance_hierarchy THEN
        PERFORM pg_notify('tags_changed', OLD.agent_instance_hierarchy);
    END IF;
    IF NEW.agent_instance_hierarchy IS NOT NULL THEN
        PERFORM pg_notify('tags_changed', NEW.agent_instance_hierarchy);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER tags_changed_notify
AFTER INSERT OR UPDATE OR DELETE ON objectiveai.tags
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_tags_changed();

-- `laboratory_attachments`: laboratory IDs attached to an agent target.
-- The target is EITHER an `agent_instance_hierarchy` (AIH) OR a `tag`
-- (never both — same exclusivity CHECK as `tags`/`message_queue`). A
-- given laboratory is attached at most once per target, enforced by a
-- partial unique index per target column. `laboratory_id` is an opaque
-- external identifier (no labs table, no FK).
CREATE TABLE IF NOT EXISTS objectiveai.laboratory_attachments (
    id                       BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy TEXT,
    tag                      TEXT,
    laboratory_id            TEXT   NOT NULL,
    -- The exact laboratory host: laboratory ids are only unique per
    -- (machine, state). NULL only on rows predating machine tracking.
    machine_id               TEXT,
    machine_state            TEXT,
    created_at               BIGINT NOT NULL,
    -- The AIH that ran the attach. NULL on rows predating tracking.
    attached_by              TEXT,
    CHECK (
        (agent_instance_hierarchy IS NOT NULL AND tag IS NULL)
        OR
        (agent_instance_hierarchy IS NULL AND tag IS NOT NULL)
    )
);
-- Existing DBs predate `attached_by` / the machine pair — align
-- idempotently.
ALTER TABLE objectiveai.laboratory_attachments
    ADD COLUMN IF NOT EXISTS attached_by TEXT;
ALTER TABLE objectiveai.laboratory_attachments
    ADD COLUMN IF NOT EXISTS machine_id TEXT;
ALTER TABLE objectiveai.laboratory_attachments
    ADD COLUMN IF NOT EXISTS machine_state TEXT;

-- NOTIFY on attach/detach so the daemon's per-agent status frames can
-- track the ATTACHED laboratory set live. A row targets EITHER an AIH
-- OR a tag (CHECK-exclusive), and the payload preserves which:
-- `aih:<value>` / `tag:<value>` — the daemon resolves a tag payload to
-- its bound AIH (GROUPED tags map to no live record and are dropped
-- there, matching the read path).
CREATE OR REPLACE FUNCTION objectiveai.notify_laboratory_attachments_changed()
RETURNS trigger AS $$
DECLARE
    row RECORD;
BEGIN
    IF (TG_OP = 'DELETE') THEN
        row := OLD;
    ELSE
        row := NEW;
    END IF;
    IF row.agent_instance_hierarchy IS NOT NULL THEN
        PERFORM pg_notify('laboratory_attachments_changed',
                          'aih:' || row.agent_instance_hierarchy);
    ELSIF row.tag IS NOT NULL THEN
        PERFORM pg_notify('laboratory_attachments_changed', 'tag:' || row.tag);
    END IF;
    RETURN row;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER laboratory_attachments_changed_notify
AFTER INSERT OR UPDATE OR DELETE ON objectiveai.laboratory_attachments
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_laboratory_attachments_changed();

-- `agent_active_laboratories`: the laboratory ids sent with an agent's
-- MOST RECENT spawn request (what `run_multi_pass` resolved for its
-- latest pass), replaced wholesale per pass. Most-recent-value
-- semantics like `agent_token_usage`: survives deactivation. There is
-- deliberately NO row trigger — the replace fires ONE
-- `agent_active_laboratories_changed` NOTIFY (payload = the AIH) from
-- Rust per transaction, not one per row.
CREATE TABLE IF NOT EXISTS objectiveai.agent_active_laboratories (
    agent_instance_hierarchy TEXT   NOT NULL,
    laboratory_id            TEXT   NOT NULL,
    -- Position within the resolved set (resolve order is meaningful:
    -- first-seen dedup order from the attachment targets).
    ordinal                  BIGINT NOT NULL,
    updated_at               BIGINT NOT NULL,
    PRIMARY KEY (agent_instance_hierarchy, laboratory_id)
);
-- Uniqueness is per FULL laboratory identity: the same id from two
-- different hosts can attach to one target. The narrow pre-machine
-- indexes are dropped (idempotently) in favor of the widened ones.
DROP INDEX IF EXISTS objectiveai.laboratory_attachments_tag_unique_idx;
DROP INDEX IF EXISTS objectiveai.laboratory_attachments_aih_unique_idx;
CREATE UNIQUE INDEX IF NOT EXISTS laboratory_attachments_tag_machine_unique_idx
    ON objectiveai.laboratory_attachments(tag, laboratory_id, machine_id, machine_state)
    WHERE tag IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS laboratory_attachments_aih_machine_unique_idx
    ON objectiveai.laboratory_attachments(agent_instance_hierarchy, laboratory_id, machine_id, machine_state)
    WHERE agent_instance_hierarchy IS NOT NULL;

-- Latest continuation token per agent_instance_hierarchy. Upserted
-- per streamed chunk by the chunk-yielder loops in `agents spawn`
-- and `functions execute`. No GC, no history — querying it gives
-- the single most recent continuation for that AIH.
CREATE TABLE IF NOT EXISTS objectiveai.agent_continuations (
    agent_instance_hierarchy TEXT PRIMARY KEY NOT NULL,
    continuation             TEXT             NOT NULL,
    updated_at               BIGINT           NOT NULL
);

-- Most-recent agent-completion `total_tokens` per AIH. Overwritten
-- (last-write-wins) by the log writer every time it encounters an
-- agent-completion chunk carrying a non-NULL usage, at any tier
-- (standalone agent completion, or nested inside vector/function
-- executions). A snapshot for sampling token usage over time — not a
-- running sum.
CREATE TABLE IF NOT EXISTS objectiveai.agent_token_usage (
    agent_instance_hierarchy TEXT   PRIMARY KEY NOT NULL,
    total_tokens             BIGINT NOT NULL
);

-- The definition SOURCE per AIH: EITHER the RemotePath the agent's WF
-- was fetched from, OR the inline WF spec itself — exactly one set.
-- Blindly upserted (last write wins, no read-before-write) from two
-- sites: `agents spawn` for spawns by SPEC at AIH-lock acquisition,
-- and the log writer whenever a chunk carries `agent_inline` (the
-- first chunk of every completion, any tier/nesting).
CREATE TABLE IF NOT EXISTS objectiveai.agent_refs (
    agent_instance_hierarchy TEXT   PRIMARY KEY NOT NULL,
    -- The remote path's canonical JSON (RemotePathCommitOptional's
    -- tagged object). JSONB like every serialized-JSON column — never
    -- JSON-in-TEXT.
    remote                   JSONB,
    inline                   JSONB,
    updated_at               BIGINT NOT NULL,
    CHECK (
        (remote IS NOT NULL AND inline IS NULL)
        OR
        (remote IS NULL AND inline IS NOT NULL)
    )
);

-- Existing DBs predate JSONB `remote` (it was TEXT holding the same
-- JSON text) — align idempotently. Legacy plain-string rows become
-- JSON strings rather than failing the cast.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'objectiveai' AND table_name = 'agent_refs'
          AND column_name = 'remote' AND data_type = 'text'
    ) THEN
        ALTER TABLE objectiveai.agent_refs
            ALTER COLUMN remote TYPE JSONB
            USING CASE
                WHEN remote IS NULL THEN NULL
                WHEN remote ~ '^\s*[\[{"]' THEN remote::jsonb
                ELSE to_jsonb(remote)
            END;
    END IF;
END $$;


-- AFTER-INSERT/UPDATE trigger on `agent_token_usage`: every write
-- emits `NOTIFY agent_token_usage_changed '<agent_instance_hierarchy>'`
-- so `agents logs token-usage subscribe` wakes the instant an AIH's
-- token snapshot is written. The listener re-reads and compares to a
-- baseline, so a same-value overwrite (the writer's ON CONFLICT DO
-- UPDATE fires this even when the number is unchanged) is filtered
-- out client-side.
CREATE OR REPLACE FUNCTION objectiveai.notify_agent_token_usage_changed()
RETURNS trigger AS $$
BEGIN
    PERFORM pg_notify('agent_token_usage_changed', NEW.agent_instance_hierarchy);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER agent_token_usage_changed_notify
AFTER INSERT OR UPDATE ON objectiveai.agent_token_usage
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_agent_token_usage_changed();

CREATE TABLE IF NOT EXISTS objectiveai.message_queue (
    id                              BIGSERIAL PRIMARY KEY,
    agent_instance_hierarchy        TEXT,
    agent_tag                       TEXT,
    -- AIH of the caller who enqueued this row (sourced from
    -- `scoped.agent_instance_hierarchy()` at enqueue time).
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
    ON objectiveai.message_queue(agent_instance_hierarchy, id)
    WHERE agent_instance_hierarchy IS NOT NULL;
CREATE INDEX IF NOT EXISTS message_queue_tag_idx
    ON objectiveai.message_queue(agent_tag, id)
    WHERE agent_tag IS NOT NULL;
-- Per-target idempotency keys. The `AND active = TRUE` clause
-- means inactive prior rows don't count toward uniqueness — an
-- `agents message --enqueue-with-key k` after a prior consumption
-- inserts cleanly without UNIQUE-violating the soft-flipped row.
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_hierarchy_unique_idx
    ON objectiveai.message_queue(agent_instance_hierarchy, key)
    WHERE agent_instance_hierarchy IS NOT NULL
      AND key IS NOT NULL
      AND active = TRUE;
CREATE UNIQUE INDEX IF NOT EXISTS message_queue_key_tag_unique_idx
    ON objectiveai.message_queue(agent_tag, key)
    WHERE agent_tag IS NOT NULL
      AND key IS NOT NULL
      AND active = TRUE;

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_contents (
    id               BIGSERIAL PRIMARY KEY,
    message_queue_id BIGINT NOT NULL,
    kind             TEXT   NOT NULL
        CHECK (kind IN ('text','image','audio','video','file')),
    FOREIGN KEY (message_queue_id) REFERENCES objectiveai.message_queue(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS message_queue_contents_parent_idx
    ON objectiveai.message_queue_contents(message_queue_id);

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_texts (
    id   BIGINT PRIMARY KEY,
    text TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES objectiveai.message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_images (
    id     BIGINT PRIMARY KEY,
    url    TEXT   NOT NULL,
    detail TEXT,
    FOREIGN KEY (id) REFERENCES objectiveai.message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_audios (
    id     BIGINT PRIMARY KEY,
    data   TEXT   NOT NULL,
    format TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES objectiveai.message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_videos (
    id  BIGINT PRIMARY KEY,
    url TEXT   NOT NULL,
    FOREIGN KEY (id) REFERENCES objectiveai.message_queue_contents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS objectiveai.message_queue_files (
    id        BIGINT PRIMARY KEY,
    file_data TEXT,
    file_id   TEXT,
    filename  TEXT,
    file_url  TEXT,
    FOREIGN KEY (id) REFERENCES objectiveai.message_queue_contents(id) ON DELETE CASCADE
);

-- AFTER-UPDATE trigger on `message_queue.active`: every soft-flip
-- (TRUE → FALSE) emits a `NOTIFY message_queue_inactive '<id>'`
-- so the cli's `db::message_queue::subscribe_delivered` listener
-- wakes up the instant a consumption flip lands. Pure native
-- LISTEN/NOTIFY — no polling. We no longer hard-delete, so the
-- prior AFTER DELETE trigger is gone.
CREATE OR REPLACE FUNCTION objectiveai.notify_message_queue_inactive()
RETURNS trigger AS $$
BEGIN
    IF OLD.active = TRUE AND NEW.active = FALSE THEN
        PERFORM pg_notify('message_queue_inactive', NEW.id::text);
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER message_queue_inactive_notify
AFTER UPDATE OF active ON objectiveai.message_queue
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_message_queue_inactive();

-- Per-plugin Postgres compartment credentials. One random, STORED
-- password per plugin (owner/name lowercased, version byte-for-byte),
-- minted on first ephemeral plugin create and reused thereafter. See
-- db/compartment.rs::resolve_plugin_db_credential.
CREATE TABLE IF NOT EXISTS objectiveai.plugin_db_credentials (
    owner TEXT NOT NULL,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    password TEXT NOT NULL,
    PRIMARY KEY (owner, name, version)
);


-- Durable scheduled commands (`tasks create` / `list` / `delete`).
--
-- One row per task, not per occurrence. `state` is TEXT + CHECK (not
-- a PG enum), the channels convention:
--
--   scheduled  armed; fires when next_run_at is due
--   running    claimed by a scheduler fire (in flight)
--   complete   will never fire again; stays listed until deleted
--
-- The fire CLAIM is a single atomic UPDATE (scheduled -> running,
-- WHERE next_run_at is due), which serializes concurrent schedulers —
-- including two daemons pointed at one shared remote Postgres — so a
-- task can never double-fire or overlap itself (a running task is
-- never due). Completion is a single atomic UPDATE (running ->
-- scheduled|complete). Rows found in 'running' at daemon boot are a
-- crash's orphans: reconciled back to 'scheduled', due immediately —
-- execution is AT-LEAST-ONCE and run_count counts COMPLETED runs.
--
-- Repeat semantics: errored runs (last stream item was an error) do
-- NOT consume the repeat_count budget — a permanently-failing counted
-- repeat retries forever, BY DESIGN.
--
-- `command` is the stored command-request JSON. Like the channels
-- secrets, it is readable by the reader group; it may embed secrets
-- the creator put in command arguments — same exposure class.
--
-- A task's SOLE identity is `(plugin trio, id)` — no surrogate key.
-- `id` is the USER-CHOSEN name, laboratory-style, REQUIRED and unique
-- PER CREATING PLUGIN: the namespace is the creator's plugin trio,
-- and non-plugin creators (all-NULL trio) share one namespace — hence
-- NULLS NOT DISTINCT, without which every NULL-trio row would be
-- "distinct" and the shared namespace unenforced. The trio is
-- ALL-OR-NOTHING (the CHECK below): always stamped as a set, so the
-- only two namespace shapes are a full trio and all-NULL — no partial
-- trio can create an ambiguous half-NULL namespace. That composite is
-- also what the scheduler claims/completes by (matched with
-- IS NOT DISTINCT FROM), so there is exactly ONE way to name a task.
-- (No PRIMARY KEY: a PK can't contain the nullable trio; the unique
-- constraint is the identity.)
CREATE TABLE IF NOT EXISTS objectiveai.tasks (
    id                       TEXT    NOT NULL,
    -- The creator identity the fired run reconstructs, one column per
    -- field (no JSON blob). The plugin trio doubles as the id's
    -- namespace (plain identity, not authentication).
    plugin_owner             TEXT,
    plugin_name              TEXT,
    plugin_version           TEXT,
    agent_instance_hierarchy TEXT    NOT NULL,
    agent_id                 TEXT,
    agent_full_id            TEXT,
    agent_remote             TEXT,
    response_id              TEXT,
    response_ids             TEXT,
    command                  JSONB   NOT NULL,
    delay_secs               BIGINT  NOT NULL,
    repeat                   BOOLEAN NOT NULL,
    repeat_count             BIGINT,
    run_count                BIGINT  NOT NULL DEFAULT 0,
    error_count              BIGINT  NOT NULL DEFAULT 0,
    last_result              TEXT    CHECK (last_result IN ('success', 'error')),
    state                    TEXT    NOT NULL CHECK (state IN ('scheduled', 'running', 'complete')),
    created_at               BIGINT  NOT NULL,
    next_run_at              BIGINT,
    -- Plugin trio is all-set or all-NULL — no partial namespaces.
    CHECK (
        (plugin_owner IS NULL AND plugin_name IS NULL AND plugin_version IS NULL)
        OR (plugin_owner IS NOT NULL AND plugin_name IS NOT NULL AND plugin_version IS NOT NULL)
    ),
    UNIQUE NULLS NOT DISTINCT (plugin_owner, plugin_name, plugin_version, id)
);


-- =====================================================================
-- objectiveai-cli `objectiveai.*` schema — hybrid blob + streaming.
-- =====================================================================
--
-- Design:
--
-- 1. *Six tier tables* (request + response × agent/vector/function)
--    store the FULL chunk body as a JSONB blob. Requests are written
--    once on first chunk arrival. Responses are written on every tick
--    via UPSERT — postgres's MVCC means the blob is rewritten, but
--    UPSERT with PK = response_id is the simplest write path and the
--    shadow map in the writer skips no-op rewrites.
--
-- 2. *Fourteen streaming-content tables* hold the per-message,
--    per-part incrementally-updating content. Their PKs are
--    deterministic tuples of (response_id, index[, sub_index]) so
--    diff-by-key is trivial: the writer compares the new row body to
--    the last-written shadow and UPSERTs only on change.
--
-- The streaming-content tables track ALL embedded agent completions
-- recursively — vector completions surface their per-agent completion
-- chunks, function executions surface every nested vector / function /
-- reasoning agent completion. Because every agent completion has a
-- globally-unique `response_id`, the writer can dispatch every yielded
-- row through the same shared UPSERT path regardless of which
-- enclosing chunk produced it.
-- =====================================================================
-- Tier blobs (6)
-- =====================================================================
-- The chunk body (response side) or create-params body (request side)
-- is serialized to JSONB and stored verbatim. PK = wire response_id.

-- Neither request nor response blobs carry agent_instance_hierarchy
-- directly. A single request can be the entry point for many agents
-- (vector / function tiers; nested function tasks), so the "which
-- agent is this for?" linkage lives in `objectiveai.messages` — one row
-- there per (request_response_id, agent_instance_hierarchy) pair,
-- written by the LogWriter the first time it sees each agent in the
-- chunk's row iterator. All three tiers share the same blob shape.
CREATE TABLE IF NOT EXISTS objectiveai.agent_completion_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    -- AIH of the caller who issued this completion request
    -- (from the request scope's `agent_instance_hierarchy` at request
    -- time). Denormalized into `objectiveai.messages.sender_*` for
    -- fast filtering on the read-all path.
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acreq_body_gin
    ON objectiveai.agent_completion_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS objectiveai.agent_completion_responses (
    response_id              TEXT PRIMARY KEY,
    body                     JSONB NOT NULL,
    created_at               BIGINT NOT NULL,
    inserted_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acresp_body_gin
    ON objectiveai.agent_completion_responses USING GIN (body);

CREATE TABLE IF NOT EXISTS objectiveai.vector_completion_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_vcreq_body_gin
    ON objectiveai.vector_completion_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS objectiveai.vector_completion_responses (
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_vcresp_body_gin
    ON objectiveai.vector_completion_responses USING GIN (body);

CREATE TABLE IF NOT EXISTS objectiveai.function_execution_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_fereq_body_gin
    ON objectiveai.function_execution_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS objectiveai.function_execution_responses (
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_feresp_body_gin
    ON objectiveai.function_execution_responses USING GIN (body);

-- =====================================================================
-- Streaming content: tool response container (1)
-- =====================================================================
-- One row per tool response message embedded in an agent completion
-- chunk's messages array. Keyed by (response_id, index) — the index
-- is the message's position within `messages[]`.

CREATE TABLE IF NOT EXISTS objectiveai.tool_response (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    tool_call_id TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

-- =====================================================================
-- Streaming content: assistant scalar slots (3)
-- =====================================================================

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_refusal (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_reasoning (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

-- One row per tool call within an assistant response. `index` is the
-- assistant message's position in `messages[]`; `tool_call_index` is
-- the call's position within the message's `tool_calls[]`.
CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_tool_calls (
    response_id     TEXT   NOT NULL,
    "index"         BIGINT NOT NULL,
    tool_call_index BIGINT NOT NULL,
    tool_call_id    TEXT   NOT NULL,
    function_name   TEXT   NOT NULL,
    arguments       TEXT   NOT NULL,
    inserted_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", tool_call_index)
);

-- =====================================================================
-- Streaming content: assistant content parts (5)
-- =====================================================================
-- Keyed by (response_id, index, part_index). `index` = position of
-- the owning assistant message within `messages[]`; `part_index` =
-- position of this part within the message's rich content array.

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.assistant_response_content_file (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    file_data   TEXT   NULL,
    file_id     TEXT   NULL,
    filename    TEXT   NULL,
    file_url    TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: tool response content parts (5)
-- =====================================================================
-- Same shape as the assistant variants, parented to `tool_response`
-- rows via shared (response_id, index).

CREATE TABLE IF NOT EXISTS objectiveai.tool_response_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.tool_response_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.tool_response_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.tool_response_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.tool_response_content_file (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    file_data   TEXT   NULL,
    file_id     TEXT   NULL,
    filename    TEXT   NULL,
    file_url    TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: request_message user content parts (5)
-- =====================================================================
-- The `user`-role messages of the request/task input, unpacked into
-- content parts. Separate tables from the response content so the
-- (response_id, index, part_index) key never collides with a response
-- message at the same position. `index` = the user message's position
-- within the request's `messages[]`.

CREATE TABLE IF NOT EXISTS objectiveai.request_message_user_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_user_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_user_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_user_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_user_content_file (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    file_data   TEXT   NULL,
    file_id     TEXT   NULL,
    filename    TEXT   NULL,
    file_url    TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: request_message assistant (mirror of response, 8)
-- =====================================================================
-- The `assistant`-role messages of the request/task input — same
-- shape as `assistant_response_*`, distinct tables so keys never
-- collide with the response's assistant messages.

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_refusal (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_reasoning (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_tool_calls (
    response_id     TEXT   NOT NULL,
    "index"         BIGINT NOT NULL,
    tool_call_index BIGINT NOT NULL,
    tool_call_id    TEXT   NOT NULL,
    function_name   TEXT   NOT NULL,
    arguments       TEXT   NOT NULL,
    inserted_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", tool_call_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_assistant_content_file (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    file_data   TEXT   NULL,
    file_id     TEXT   NULL,
    filename    TEXT   NULL,
    file_url    TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: request_message tool (mirror of tool_response, 6)
-- =====================================================================
-- `tool`-role messages of the request/task input. Head row is the
-- `tool_call_id` lookup for its content rows (JOINed at read time) and
-- emits no messages event — same pattern as `tool_response`.

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    tool_call_id TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_message_tool_content_file (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    file_data   TEXT   NULL,
    file_id     TEXT   NULL,
    filename    TEXT   NULL,
    file_url    TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: vector request choices (head + 5 content)
-- =====================================================================
-- A function-execution vector task's response choices. The head row
-- carries this agent's inline voting `key` per choice and is the
-- JOIN target for the content rows (no messages event of its own,
-- like `tool_response`). Content rows are keyed by
-- (response_id, choice_index, part_index).

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    -- This agent's prefix-tree voting key for the choice.
    key          TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice_content_text (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    part_index   BIGINT NOT NULL,
    text         TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice_content_image (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    part_index   BIGINT NOT NULL,
    url          TEXT   NOT NULL,
    detail       TEXT   NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice_content_audio (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    part_index   BIGINT NOT NULL,
    data         TEXT   NOT NULL,
    format       TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice_content_video (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    part_index   BIGINT NOT NULL,
    url          TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS objectiveai.request_vector_choice_content_file (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    part_index   BIGINT NOT NULL,
    file_data    TEXT   NULL,
    file_id      TEXT   NULL,
    filename     TEXT   NULL,
    file_url     TEXT   NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

-- =====================================================================
-- Streaming content: vector response vote (1, inline closer)
-- =====================================================================
-- The per-agent vote (this agent's score for each choice, in choice
-- order) for a function-execution vector task. Inline JSONB array,
-- returned directly by `read all` (no `read id`). Keyed per agent.

CREATE TABLE IF NOT EXISTS objectiveai.response_vector_vote (
    response_id              TEXT   NOT NULL,
    agent_instance_hierarchy TEXT   NOT NULL,
    -- JSONB array of decimals (as strings), one per choice.
    vote                     JSONB  NOT NULL,
    inserted_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, agent_instance_hierarchy)
);

-- =====================================================================
-- messages: per-row event log
-- =====================================================================
--
-- One row per (response_id, "table", row_index, row_sub_index) — the
-- writer INSERTs exactly once per logical row, on its initial
-- streaming-content INSERT or request-blob INSERT. The `"index"`
-- column is drawn from `objectiveai.messages_index_seq` at that moment and
-- never bumps thereafter — it's the position of this event in the
-- agent's full history, the monotonic watermark that
-- `messages_queue.read_index` paginates against. Updates re-deliver
-- to readers via the messages_queue downgrade path (walking a
-- caller's read_index BACK to this row's index) instead of producing
-- duplicate events.
--
-- `row_index` and `row_sub_index` are NULL when the target table
-- doesn't carry that level of identity (request blobs have no row
-- identity beyond response_id; tool_response / assistant_response_*
-- scalar slots have only `index`). The CHECK constraint enforces the
-- per-table shape. The UNIQUE NULLS NOT DISTINCT constraint treats
-- the NULL slots as equal, so the writer's INSERT path conflicts
-- correctly with prior rows.
--
-- `objectiveai.message_table` enumerates every `objectiveai.*` table the writer
-- emits a messages row for. The three response-blob tables
-- (agent / vector / function `_responses`) are intentionally absent
-- — they're not events, just the latest snapshot.
--
-- Postgres enums have no `IF NOT EXISTS` shortcut; we wrap in a DO
-- block that swallows `duplicate_object`.
DO $logs_message_table_bootstrap$ BEGIN
    CREATE TYPE objectiveai.message_table AS ENUM (
        'agent_completion_request',
        'vector_completion_request',
        'function_execution_request',
        'message_queue_text',
        'message_queue_image',
        'message_queue_audio',
        'message_queue_video',
        'message_queue_file',
        'assistant_response_refusal',
        'assistant_response_reasoning',
        'assistant_response_tool_calls',
        'assistant_response_content_text',
        'assistant_response_content_image',
        'assistant_response_content_audio',
        'assistant_response_content_video',
        'assistant_response_content_file',
        'tool_response_content_text',
        'tool_response_content_image',
        'tool_response_content_audio',
        'tool_response_content_video',
        'tool_response_content_file',
        'request_message_user_content_text',
        'request_message_user_content_image',
        'request_message_user_content_audio',
        'request_message_user_content_video',
        'request_message_user_content_file',
        'request_message_assistant_refusal',
        'request_message_assistant_reasoning',
        'request_message_assistant_tool_calls',
        'request_message_assistant_content_text',
        'request_message_assistant_content_image',
        'request_message_assistant_content_audio',
        'request_message_assistant_content_video',
        'request_message_assistant_content_file',
        'request_message_tool_content_text',
        'request_message_tool_content_image',
        'request_message_tool_content_audio',
        'request_message_tool_content_video',
        'request_message_tool_content_file',
        'request_vector_choice_content_text',
        'request_vector_choice_content_image',
        'request_vector_choice_content_audio',
        'request_vector_choice_content_video',
        'request_vector_choice_content_file',
        'response_vector_vote',
        'error'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $logs_message_table_bootstrap$;

-- Additive enum values for pre-existing DBs (the CREATE TYPE above is
-- skipped once the type exists). `ADD VALUE IF NOT EXISTS` is a no-op
-- when the value is already present.
DO $logs_message_table_extend$ BEGIN
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_user_content_text';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_user_content_image';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_user_content_audio';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_user_content_video';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_user_content_file';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_refusal';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_reasoning';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_tool_calls';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_content_text';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_content_image';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_content_audio';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_content_video';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_assistant_content_file';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_tool_content_text';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_tool_content_image';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_tool_content_audio';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_tool_content_video';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_message_tool_content_file';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_vector_choice_content_text';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_vector_choice_content_image';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_vector_choice_content_audio';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_vector_choice_content_video';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'request_vector_choice_content_file';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'response_vector_vote';
    ALTER TYPE objectiveai.message_table ADD VALUE IF NOT EXISTS 'error';
END $logs_message_table_extend$;

-- Logged failures: one row per error the CLI persisted into an agent's
-- history (spawn-path failures after the AIH became known — see
-- `command/agents/spawn.rs`). `error` holds the CLI's user-facing
-- error value (`Error::output_message()`): a structured object for API
-- response errors, a plain string otherwise. `response_id` is the
-- response the failure belongs to when one existed, NULL for
-- post-lock pre-stream failures. The BIGSERIAL `id` doubles as the
-- `objectiveai.messages.row_index` — per-row identity that never
-- collides even when `response_id` is NULL.
CREATE TABLE IF NOT EXISTS objectiveai.errors (
    id                       BIGSERIAL           PRIMARY KEY,
    agent_instance_hierarchy TEXT                NOT NULL,
    response_id              TEXT                NULL,
    error                    JSONB               NOT NULL,
    inserted_at              TIMESTAMPTZ         NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS objectiveai.messages (
    -- NULL only on `error` rows logged before any response existed
    -- for the attempt; the row-consistency constraint (attached
    -- below the table) enforces NOT NULL for every other kind.
    response_id              TEXT                NULL,
    "table"                  objectiveai.message_table  NOT NULL,
    row_index                BIGINT              NULL,
    row_sub_index            BIGINT              NULL,
    -- Position of this event in the agent's full history (and in the
    -- global cross-agent event stream — the column is BIGSERIAL, so
    -- per-agent values are gappy but strictly increasing). Assigned
    -- once on INSERT via the implicit sequence postgres creates for
    -- BIGSERIAL (`objectiveai.messages_index_seq`); never bumped on UPDATE.
    -- Readers paginate `WHERE "index" > read_index`; the writer
    -- downgrades `messages_queue.read_index` when an UPDATE needs to
    -- re-deliver an already-consumed row to a caller.
    "index"                  BIGSERIAL           NOT NULL,
    -- Every messages row is per-agent. For streaming-content kinds
    -- this is the agent that produced the chunk; for request-blob
    -- kinds it's the agent whose history is being seeded with "the
    -- request was made for me" — the writer emits one such row per
    -- agent it sees in the chunk's row iterator.
    --
    -- `sender_agent_instance_hierarchy` is NOT stored here — the
    -- read-all path JOINs to the row's source table by `response_id`
    -- (for request blob / assistant_response_* / tool_response*
    -- rows → the matching `objectiveai.<tier>_completion_requests` /
    -- `objectiveai.function_execution_requests` table) or via
    -- `row_index` → `message_queue_contents.id` → `message_queue`
    -- (for `message_queue_*` rows). Same story for `timestamp_queued`
    -- on ClientNotification parts — derived from `message_queue.enqueued_at`
    -- at read time, not duplicated.
    agent_instance_hierarchy TEXT                NOT NULL,
    "timestamp"              BIGINT              NOT NULL,
    UNIQUE NULLS NOT DISTINCT (response_id, "table", row_index, row_sub_index, agent_instance_hierarchy)
);
-- Existing DBs predate nullable `response_id` — align idempotently
-- (a no-op when already nullable).
ALTER TABLE objectiveai.messages ALTER COLUMN response_id DROP NOT NULL;
-- The row-shape consistency constraint is attached OUTSIDE the CREATE
-- TABLE (drop + re-add) so schema evolution reaches pre-existing DBs
-- — `CREATE TABLE IF NOT EXISTS` never revisits an existing table's
-- constraints. Every kind comparison is on `"table"::text`, NOT enum
-- literals: an enum value added by the extend block above cannot be
-- referenced as an enum literal in the same transaction (the whole
-- schema applies as one batch), and text comparison sidesteps that.
ALTER TABLE objectiveai.messages DROP CONSTRAINT IF EXISTS messages_table_row_consistency;
ALTER TABLE objectiveai.messages ADD CONSTRAINT messages_table_row_consistency CHECK (
    -- `error` rows: the ONLY kind allowed a NULL response_id
    -- (post-lock pre-stream failures have no response). row_index =
    -- `objectiveai.errors.id` — identity that never collides even
    -- with response_id NULL; no sub-index.
    ("table"::text = 'error'
     AND row_index IS NOT NULL AND row_sub_index IS NULL)
    OR
    (response_id IS NOT NULL AND (
        -- No-per-row-identity kinds: the request blobs, plus the
        -- per-agent vector vote (keyed by response_id +
        -- agent_instance_hierarchy, both already on this row).
        ("table"::text IN (
            'agent_completion_request',
            'vector_completion_request',
            'function_execution_request',
            'response_vector_vote'
         )
         AND row_index IS NULL AND row_sub_index IS NULL)
        OR
        -- Single-index kinds: assistant refusal / reasoning (response
        -- and request), and the five `message_queue_*` kinds (row_index =
        -- `message_queue_contents.id`; per-kind enum value picks the
        -- corresponding per-kind table `message_queue_texts` /
        -- `_images` / `_audios` / `_videos` / `_files` directly at
        -- read time). row_index only — no sub_index.
        ("table"::text IN (
            'message_queue_text',
            'message_queue_image',
            'message_queue_audio',
            'message_queue_video',
            'message_queue_file',
            'assistant_response_refusal',
            'assistant_response_reasoning',
            'request_message_assistant_refusal',
            'request_message_assistant_reasoning'
         )
         AND row_index IS NOT NULL AND row_sub_index IS NULL)
        OR
        -- Sub-indexed streaming kinds (tool calls + every content
        -- table, response and request): both indices required. For the
        -- request_vector_choice_content_* kinds, row_index = choice
        -- index and row_sub_index = part index.
        ("table"::text IN (
            'assistant_response_tool_calls',
            'assistant_response_content_text',
            'assistant_response_content_image',
            'assistant_response_content_audio',
            'assistant_response_content_video',
            'assistant_response_content_file',
            'tool_response_content_text',
            'tool_response_content_image',
            'tool_response_content_audio',
            'tool_response_content_video',
            'tool_response_content_file',
            'request_message_user_content_text',
            'request_message_user_content_image',
            'request_message_user_content_audio',
            'request_message_user_content_video',
            'request_message_user_content_file',
            'request_message_assistant_tool_calls',
            'request_message_assistant_content_text',
            'request_message_assistant_content_image',
            'request_message_assistant_content_audio',
            'request_message_assistant_content_video',
            'request_message_assistant_content_file',
            'request_message_tool_content_text',
            'request_message_tool_content_image',
            'request_message_tool_content_audio',
            'request_message_tool_content_video',
            'request_message_tool_content_file',
            'request_vector_choice_content_text',
            'request_vector_choice_content_image',
            'request_vector_choice_content_audio',
            'request_vector_choice_content_video',
            'request_vector_choice_content_file'
         )
         AND row_index IS NOT NULL AND row_sub_index IS NOT NULL)
    ))
);
CREATE INDEX IF NOT EXISTS messages_index_idx
    ON objectiveai.messages("index");
CREATE INDEX IF NOT EXISTS messages_response_index_idx
    ON objectiveai.messages(response_id, "index");
CREATE INDEX IF NOT EXISTS messages_agent_hier_index_idx
    ON objectiveai.messages(agent_instance_hierarchy, "index");

-- Fires NOTIFY on every new objectiveai.messages row. Payload is the
-- producing agent's `agent_instance_hierarchy` — subscribers
-- (`agents logs read subscribe`) filter on AIH match, then
-- re-call `read_pending_for_parent` to drain anything new.
-- Keeps the trigger stupid: no per-kind filtering here, all the
-- type filtering happens in the read query.
CREATE OR REPLACE FUNCTION objectiveai.notify_messages_inserted()
RETURNS trigger AS $logs_msg_notify$
BEGIN
    PERFORM pg_notify(
        'logs_messages_inserted',
        NEW.agent_instance_hierarchy
    );
    RETURN NEW;
END;
$logs_msg_notify$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER logs_messages_inserted_notify
AFTER INSERT ON objectiveai.messages
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_messages_inserted();

-- =====================================================================
-- messages_queue: per-caller read watermark
-- =====================================================================
--
-- One row per (parent caller, spawned agent) pair. Each caller's
-- `read_index` advances as the caller consumes events from the
-- spawned agent's stream:
--
--     SELECT … FROM objectiveai.messages
--     WHERE agent_instance_hierarchy = $spawned
--       AND "index" > $read_index
--     ORDER BY "index";
--
-- The writer never INSERTs into this table — callers are responsible
-- for creating their own subscription rows when they start reading.
-- The writer's role: on every streaming-content UPDATE (a non-blob
-- write that targets an already-existing logical row), it downgrades
-- any caller whose `read_index >= messages."index"` for the affected
-- spawned agent. Because `"index"` stays fixed at the row's original
-- INSERT, the only way to re-deliver an updated row to a caller who
-- has already moved past it is to walk their watermark back.
CREATE TABLE IF NOT EXISTS objectiveai.messages_queue (
    parent_agent_instance_hierarchy  TEXT   NOT NULL,
    spawned_agent_instance_hierarchy TEXT   NOT NULL,
    read_index                       BIGINT NOT NULL,
    PRIMARY KEY (parent_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
);
CREATE INDEX IF NOT EXISTS messages_queue_spawned_idx
    ON objectiveai.messages_queue(spawned_agent_instance_hierarchy);


-- =====================================================================
-- log_reader role: read-only access for the future LLM SQL endpoint.
-- =====================================================================
DO $logs_role_bootstrap$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'log_reader') THEN
        CREATE ROLE log_reader NOLOGIN;
    END IF;
END $logs_role_bootstrap$;
REVOKE ALL ON SCHEMA objectiveai FROM PUBLIC;
REVOKE ALL ON ALL TABLES IN SCHEMA objectiveai FROM PUBLIC;
GRANT USAGE ON SCHEMA objectiveai TO log_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA objectiveai TO log_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA objectiveai GRANT SELECT ON TABLES TO log_reader;
ALTER ROLE log_reader SET search_path = objectiveai;

-- `detail` holds the BARE enum token (`auto`/`low`/`high`). Early
-- writers stored the JSON-quoted form (`"auto"`), which no reader
-- parses — strip idempotently (fixed rows never match the filter).
UPDATE objectiveai.assistant_response_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';
UPDATE objectiveai.tool_response_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';
UPDATE objectiveai.request_message_user_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';
UPDATE objectiveai.request_message_assistant_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';
UPDATE objectiveai.request_message_tool_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';
UPDATE objectiveai.request_vector_choice_content_image SET detail = detail::jsonb #>> '{}' WHERE detail LIKE '"%';


-- =====================================================================
-- objectiveai `channels` schema — durable duplex channels + message log.
-- =====================================================================
--
-- A CHANNEL is a two-party, capability-gated, append-only message log
-- between a PUBLISHER (a trusted `/execute` caller) and an OWNER (an
-- SSE client that accepted the offer). The pre-accept OFFER is
-- ephemeral in-memory coordination (see `http::channel_routes`) — the
-- durable row here is created only on ACCEPT, when the channel becomes
-- a real two-party thing.
--
-- Lifecycle: a channel is born `open`; when the owner's SSE connection
-- drops it goes `closed` (terminal — no further requests/replies), but
-- the row and its log LINGER (readable, listable) until an explicit
-- delete. Daemon shutdown never touches this history.
--
-- `state`/`direction` are TEXT+CHECK rather than PG enums — the value
-- space is tiny and stable, and this keeps the layer free of enum
-- bootstrap ceremony.
-- One durable row per accepted channel.
CREATE TABLE IF NOT EXISTS objectiveai.channels (
    id                TEXT   PRIMARY KEY,
    -- The publisher's per-channel capability (`S_pub`), returned to
    -- the `channels publish` command. Authorizes `logs request` and
    -- publisher-side reads.
    pub_secret        TEXT   NOT NULL,
    -- The owner's per-channel capability (`S_owner`), pushed to the
    -- accepting SSE connection. Authorizes `logs reply` and
    -- owner-side reads.
    owner_secret      TEXT   NOT NULL,
    state             TEXT   NOT NULL CHECK (state IN ('open', 'closed')),
    -- The offer discriminator + payload the publisher broadcast.
    key               TEXT   NOT NULL,
    details           JSONB  NOT NULL,
    -- Human-readable offer message the publisher broadcast.
    message           TEXT   NOT NULL,
    -- The originating plugin (unspoofable; stamped by `plugins run`),
    -- absent when the caller wasn't a plugin.
    plugin_owner      TEXT,
    plugin_name       TEXT,
    plugin_version    TEXT,
    -- The publisher's agent identity, from its scope.
    agent_arguments   JSONB  NOT NULL,
    -- Per-role read watermarks over `channel_messages.id`. Publisher
    -- reads REPLIES past `pub_read_index`; owner reads REQUESTS past
    -- `owner_read_index`. Bumped monotonically by `--pending`/subscribe.
    pub_read_index    BIGINT NOT NULL DEFAULT 0,
    owner_read_index  BIGINT NOT NULL DEFAULT 0,
    -- Unix-seconds (surfaced as RFC3339 via `db::time`).
    created_at        BIGINT NOT NULL,
    closed_at         BIGINT
);

-- The append-only per-channel message log. `id` BIGSERIAL is the
-- global position AND the per-channel cursor (`--after-id`, watermarks).
-- `content` is the arbitrary-JSON payload revealed only by `logs open`;
-- everything else is the envelope `logs list` returns.
CREATE TABLE IF NOT EXISTS objectiveai.channel_messages (
    id           BIGSERIAL PRIMARY KEY,
    channel_id   TEXT   NOT NULL REFERENCES objectiveai.channels(id) ON DELETE CASCADE,
    -- 'request' = publisher -> owner; 'reply' = owner -> publisher.
    direction    TEXT   NOT NULL CHECK (direction IN ('request', 'reply')),
    -- The writer's agent identity, daemon-authored from its scope.
    identity     JSONB  NOT NULL,
    content      JSONB  NOT NULL,
    delivered_at BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS channel_messages_channel_id_idx
    ON objectiveai.channel_messages(channel_id, id);

-- Fires NOTIFY on every new message. Payload = the channel id, so a
-- `channels logs subscribe` waiter filters on its channel then
-- re-reads pending.
CREATE OR REPLACE FUNCTION objectiveai.notify_channel_message_inserted()
RETURNS trigger AS $channel_msg_notify$
BEGIN
    PERFORM pg_notify('channel_messages_inserted', NEW.channel_id);
    RETURN NEW;
END;
$channel_msg_notify$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER channel_messages_inserted_notify
AFTER INSERT ON objectiveai.channel_messages
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_channel_message_inserted();

-- Fires NOTIFY when a channel transitions to `closed`, so blocked
-- subscribers wake and return their terminal `ChannelClosed`.
CREATE OR REPLACE FUNCTION objectiveai.notify_channel_closed()
RETURNS trigger AS $channel_closed_notify$
BEGIN
    IF NEW.state = 'closed' AND OLD.state IS DISTINCT FROM 'closed' THEN
        PERFORM pg_notify('channel_closed', NEW.id);
    END IF;
    RETURN NEW;
END;
$channel_closed_notify$ LANGUAGE plpgsql;
CREATE OR REPLACE TRIGGER channel_closed_notify
AFTER UPDATE OF state ON objectiveai.channels
FOR EACH ROW EXECUTE FUNCTION objectiveai.notify_channel_closed();
