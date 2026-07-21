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

