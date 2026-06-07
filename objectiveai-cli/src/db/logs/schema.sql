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

CREATE TABLE IF NOT EXISTS logs.agent_completion_requests (
    response_id              TEXT PRIMARY KEY,
    agent_instance_hierarchy TEXT NOT NULL,
    body                     JSONB NOT NULL,
    created_at               BIGINT NOT NULL,
    inserted_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acreq_agent_instance_hierarchy
    ON logs.agent_completion_requests(agent_instance_hierarchy);
CREATE INDEX IF NOT EXISTS idx_acreq_body_gin
    ON logs.agent_completion_requests USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.agent_completion_responses (
    response_id              TEXT PRIMARY KEY,
    agent_instance_hierarchy TEXT NOT NULL,
    body                     JSONB NOT NULL,
    created_at               BIGINT NOT NULL,
    inserted_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_acresp_agent_instance_hierarchy
    ON logs.agent_completion_responses(agent_instance_hierarchy);
CREATE INDEX IF NOT EXISTS idx_acresp_body_gin
    ON logs.agent_completion_responses USING GIN (body);

CREATE TABLE IF NOT EXISTS logs.vector_completion_requests (
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
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
    response_id TEXT PRIMARY KEY,
    body        JSONB NOT NULL,
    created_at  BIGINT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT now()
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
    response_id TEXT    NOT NULL,
    "index"     BIGINT  NOT NULL,
    part_index  BIGINT  NOT NULL,
    url         TEXT    NOT NULL,
    is_input    BOOLEAN NOT NULL,
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
    response_id TEXT    NOT NULL,
    "index"     BIGINT  NOT NULL,
    part_index  BIGINT  NOT NULL,
    url         TEXT    NOT NULL,
    is_input    BOOLEAN NOT NULL,
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
