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

CREATE SCHEMA IF NOT EXISTS objectiveai;

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
    -- The originating plugin (unspoofable; stamped by `plugins run`),
    -- absent when the caller wasn't a plugin.
    plugin_owner      TEXT,
    plugin_repository TEXT,
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
