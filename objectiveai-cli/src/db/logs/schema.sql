-- =====================================================================
-- objectiveai-cli `logs.*` schema — 20 tables, hybrid blob + streaming.
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

CREATE SCHEMA IF NOT EXISTS logs;

-- =====================================================================
-- Tier blobs (6)
-- =====================================================================
-- The chunk body (response side) or create-params body (request side)
-- is serialized to JSONB and stored verbatim. PK = wire response_id.

-- Neither request nor response blobs carry agent_instance_hierarchy
-- directly. A single request can be the entry point for many agents
-- (vector / function tiers; nested function tasks), so the "which
-- agent is this for?" linkage lives in `logs.messages` — one row
-- there per (request_response_id, agent_instance_hierarchy) pair,
-- written by the LogWriter the first time it sees each agent in the
-- chunk's row iterator. All three tiers share the same blob shape.
CREATE TABLE IF NOT EXISTS logs.agent_completion_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    -- AIH of the caller who issued this completion request
    -- (from `ctx.config.agent_instance_hierarchy` at request
    -- time). Denormalized into `logs.messages.sender_*` for
    -- fast filtering on the read-all path.
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acreq_body_gin
    ON logs.agent_completion_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.agent_completion_responses (
    response_id              TEXT PRIMARY KEY,
    body                     JSONB NOT NULL,
    created_at               BIGINT NOT NULL,
    inserted_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acresp_body_gin
    ON logs.agent_completion_responses USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.vector_completion_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_vcreq_body_gin
    ON logs.vector_completion_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.vector_completion_responses (
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_vcresp_body_gin
    ON logs.vector_completion_responses USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.function_execution_requests (
    response_id                     TEXT PRIMARY KEY,
    body                            JSONB NOT NULL,
    created_at                      BIGINT NOT NULL,
    sender_agent_instance_hierarchy TEXT NOT NULL,
    inserted_at                     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_fereq_body_gin
    ON logs.function_execution_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.function_execution_responses (
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_feresp_body_gin
    ON logs.function_execution_responses USING GIN (body);

-- =====================================================================
-- Streaming content: tool response container (1)
-- =====================================================================
-- One row per tool response message embedded in an agent completion
-- chunk's messages array. Keyed by (response_id, index) — the index
-- is the message's position within `messages[]`.

CREATE TABLE IF NOT EXISTS logs.tool_response (
    response_id  TEXT   NOT NULL,
    "index"      BIGINT NOT NULL,
    tool_call_id TEXT   NOT NULL,
    inserted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

-- =====================================================================
-- Streaming content: assistant scalar slots (3)
-- =====================================================================

CREATE TABLE IF NOT EXISTS logs.assistant_response_refusal (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

CREATE TABLE IF NOT EXISTS logs.assistant_response_reasoning (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index")
);

-- One row per tool call within an assistant response. `index` is the
-- assistant message's position in `messages[]`; `tool_call_index` is
-- the call's position within the message's `tool_calls[]`.
CREATE TABLE IF NOT EXISTS logs.assistant_response_tool_calls (
    response_id     TEXT   NOT NULL,
    "index"         BIGINT NOT NULL,
    tool_call_index BIGINT NOT NULL,
    tool_call_id    TEXT   NOT NULL,
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

CREATE TABLE IF NOT EXISTS logs.assistant_response_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.assistant_response_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.assistant_response_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.assistant_response_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.assistant_response_content_file (
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

CREATE TABLE IF NOT EXISTS logs.tool_response_content_text (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    text        TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.tool_response_content_image (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    detail      TEXT   NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.tool_response_content_audio (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    data        TEXT   NOT NULL,
    format      TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.tool_response_content_video (
    response_id TEXT   NOT NULL,
    "index"     BIGINT NOT NULL,
    part_index  BIGINT NOT NULL,
    url         TEXT   NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (response_id, "index", part_index)
);

CREATE TABLE IF NOT EXISTS logs.tool_response_content_file (
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
-- messages: per-row event log
-- =====================================================================
--
-- One row per (response_id, "table", row_index, row_sub_index) — the
-- writer INSERTs exactly once per logical row, on its initial
-- streaming-content INSERT or request-blob INSERT. The `"index"`
-- column is drawn from `logs.messages_index_seq` at that moment and
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
-- `logs.message_table` enumerates every `logs.*` table the writer
-- emits a messages row for. The three response-blob tables
-- (agent / vector / function `_responses`) are intentionally absent
-- — they're not events, just the latest snapshot.
--
-- Postgres enums have no `IF NOT EXISTS` shortcut; we wrap in a DO
-- block that swallows `duplicate_object`.
DO $logs_message_table_bootstrap$ BEGIN
    CREATE TYPE logs.message_table AS ENUM (
        'agent_completion_request',
        'vector_completion_request',
        'function_execution_request',
        'message_queue_text',
        'message_queue_image',
        'message_queue_audio',
        'message_queue_video',
        'message_queue_file',
        'tool_response',
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
        'tool_response_content_file'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $logs_message_table_bootstrap$;

CREATE TABLE IF NOT EXISTS logs.messages (
    response_id              TEXT                NOT NULL,
    "table"                  logs.message_table  NOT NULL,
    row_index                BIGINT              NULL,
    row_sub_index            BIGINT              NULL,
    -- Position of this event in the agent's full history (and in the
    -- global cross-agent event stream — the column is BIGSERIAL, so
    -- per-agent values are gappy but strictly increasing). Assigned
    -- once on INSERT via the implicit sequence postgres creates for
    -- BIGSERIAL (`logs.messages_index_seq`); never bumped on UPDATE.
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
    -- rows → the matching `logs.<tier>_completion_requests` /
    -- `logs.function_execution_requests` table) or via
    -- `row_index` → `message_queue_contents.id` → `message_queue`
    -- (for `message_queue_*` rows). Same story for `timestamp_queued`
    -- on ClientNotification parts — derived from `message_queue.enqueued_at`
    -- at read time, not duplicated.
    agent_instance_hierarchy TEXT                NOT NULL,
    "timestamp"              BIGINT              NOT NULL,
    CONSTRAINT messages_table_row_consistency CHECK (
        -- Request-blob kinds: no per-row identity.
        ("table" IN (
            'agent_completion_request',
            'vector_completion_request',
            'function_execution_request'
         )
         AND row_index IS NULL AND row_sub_index IS NULL)
        OR
        -- Single-index kinds: `tool_response`, assistant refusal
        -- / reasoning, and the five `message_queue_*` kinds
        -- (row_index = `message_queue_contents.id`; per-kind
        -- enum value picks the corresponding per-kind table
        -- `message_queue_texts` / `_images` / `_audios` /
        -- `_videos` / `_files` directly at read time). row_index
        -- only — no sub_index.
        ("table" IN (
            'message_queue_text',
            'message_queue_image',
            'message_queue_audio',
            'message_queue_video',
            'message_queue_file',
            'tool_response',
            'assistant_response_refusal',
            'assistant_response_reasoning'
         )
         AND row_index IS NOT NULL AND row_sub_index IS NULL)
        OR
        -- Sub-indexed streaming kinds (tool calls + every content
        -- table): both indices required.
        ("table" IN (
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
            'tool_response_content_file'
         )
         AND row_index IS NOT NULL AND row_sub_index IS NOT NULL)
    ),
    UNIQUE NULLS NOT DISTINCT (response_id, "table", row_index, row_sub_index, agent_instance_hierarchy)
);
CREATE INDEX IF NOT EXISTS messages_index_idx
    ON logs.messages("index");
CREATE INDEX IF NOT EXISTS messages_response_index_idx
    ON logs.messages(response_id, "index");
CREATE INDEX IF NOT EXISTS messages_agent_hier_index_idx
    ON logs.messages(agent_instance_hierarchy, "index");

-- Fires NOTIFY on every new logs.messages row. Payload is the
-- producing agent's `agent_instance_hierarchy` — subscribers
-- (`agents logs read subscribe`) filter on AIH match, then
-- re-call `read_pending_for_parent` to drain anything new.
-- Keeps the trigger stupid: no per-kind filtering here, all the
-- type filtering happens in the read query.
CREATE OR REPLACE FUNCTION logs.notify_messages_inserted()
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
AFTER INSERT ON logs.messages
FOR EACH ROW EXECUTE FUNCTION logs.notify_messages_inserted();

-- =====================================================================
-- messages_queue: per-caller read watermark
-- =====================================================================
--
-- One row per (parent caller, spawned agent) pair. Each caller's
-- `read_index` advances as the caller consumes events from the
-- spawned agent's stream:
--
--     SELECT … FROM logs.messages
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
CREATE TABLE IF NOT EXISTS logs.messages_queue (
    parent_agent_instance_hierarchy  TEXT   NOT NULL,
    spawned_agent_instance_hierarchy TEXT   NOT NULL,
    read_index                       BIGINT NOT NULL,
    PRIMARY KEY (parent_agent_instance_hierarchy, spawned_agent_instance_hierarchy)
);
CREATE INDEX IF NOT EXISTS messages_queue_spawned_idx
    ON logs.messages_queue(spawned_agent_instance_hierarchy);

-- =====================================================================
-- log_reader role: read-only access for the future LLM SQL endpoint.
-- =====================================================================
DO $logs_role_bootstrap$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'log_reader') THEN
        CREATE ROLE log_reader NOLOGIN;
    END IF;
END $logs_role_bootstrap$;
REVOKE ALL ON SCHEMA logs FROM PUBLIC;
REVOKE ALL ON ALL TABLES IN SCHEMA logs FROM PUBLIC;
GRANT USAGE ON SCHEMA logs TO log_reader;
GRANT SELECT ON ALL TABLES IN SCHEMA logs TO log_reader;
ALTER DEFAULT PRIVILEGES IN SCHEMA logs GRANT SELECT ON TABLES TO log_reader;
ALTER ROLE log_reader SET search_path = logs;
