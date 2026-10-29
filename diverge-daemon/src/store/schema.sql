-- The daemon's own schema, in the database it serves. Every statement
-- is idempotent, so this runs at every start.

CREATE SCHEMA IF NOT EXISTS diverge;

-- Nothing that is not the daemon reads these tables: a container's
-- login role lives in this same database and must never see them.
REVOKE ALL ON SCHEMA diverge FROM PUBLIC;

CREATE TABLE IF NOT EXISTS diverge.accounts (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- The identity the account is: the issuer and the subject of the
    -- token a client presents, one account per identity.
    issuer      TEXT NOT NULL,
    subject     TEXT NOT NULL,
    description TEXT,
    -- Sorted bytewise, deduplicated, rewritten whole.
    tags        TEXT[] NOT NULL DEFAULT '{}',
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The wire's Creator, as JSON.
    creator     JSONB NOT NULL,
    UNIQUE (issuer, subject)
);

CREATE TABLE IF NOT EXISTS diverge.roles (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- One role per name, and the name never changes.
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    -- The wire's grants, as JSON, in the order they were given.
    grants      JSONB NOT NULL DEFAULT '[]',
    tags        TEXT[] NOT NULL DEFAULT '{}',
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator     JSONB NOT NULL
);

-- Which account holds which role. An account deleted takes its
-- holdings with it; a role held by any account is not deleted, which
-- is the roles delete's InUse.
CREATE TABLE IF NOT EXISTS diverge.account_roles (
    account BIGINT NOT NULL REFERENCES diverge.accounts (id) ON DELETE CASCADE,
    role    BIGINT NOT NULL REFERENCES diverge.roles (id) ON DELETE RESTRICT,
    PRIMARY KEY (account, role)
);

CREATE INDEX IF NOT EXISTS account_roles_role ON diverge.account_roles (role);

-- A template is held by the hash of its own JSON; a deleted one keeps
-- its row, so that one made anew is the one that was deleted, with the
-- first create's creator and time.
CREATE TABLE IF NOT EXISTS diverge.agents_templates (
    id       TEXT PRIMARY KEY,
    template JSONB NOT NULL,
    tags     TEXT[] NOT NULL DEFAULT '{}',
    created  TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator  JSONB NOT NULL,
    deleted  BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE IF NOT EXISTS diverge.tools_templates (
    id       TEXT PRIMARY KEY,
    template JSONB NOT NULL,
    tags     TEXT[] NOT NULL DEFAULT '{}',
    created  TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator  JSONB NOT NULL,
    deleted  BOOLEAN NOT NULL DEFAULT false
);

-- The once-and-for-all index of a container among those made the same
-- way: one row per way, advanced in the transaction that makes the
-- container, so a crash mid-create never hands one number to two.
CREATE TABLE IF NOT EXISTS diverge.counters (
    key         TEXT PRIMARY KEY,
    next        BIGINT NOT NULL
);

-- The vault: keys, each holding one or more JSON values, handed to
-- the containers whose records bind the key to a secret. A key some
-- record binds is not deleted, which the records say, not a
-- constraint: the binding is inside a record's `secrets`.
CREATE TABLE IF NOT EXISTS diverge.vault (
    key     TEXT PRIMARY KEY,
    tags    TEXT[] NOT NULL DEFAULT '{}',
    created TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator JSONB NOT NULL
);

-- A key's values: its slots, numbered from 0 in the order appended,
-- gone with the key. A slot a running container holds names the
-- holder — the record's Creator, as JSON — and is NULL while free; a
-- lock is on a value, never on a key.
CREATE TABLE IF NOT EXISTS diverge.vault_slots (
    key    TEXT NOT NULL REFERENCES diverge.vault (key) ON DELETE CASCADE,
    slot   INT NOT NULL,
    value  JSONB NOT NULL,
    holder JSONB,
    PRIMARY KEY (key, slot)
);

-- An agent: a template plus what its create added. The container is
-- work made from the row, never the row.
CREATE TABLE IF NOT EXISTS diverge.agents (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    template              TEXT NOT NULL,
    index                 BIGINT NOT NULL,
    name                  TEXT UNIQUE,
    -- The provider it is pinned to and that provider's volumes
    -- mounted, as the create named them. No account: an agent holds
    -- none, and asks the daemon nothing.
    provider              JSONB,
    fuse_file_mounts      JSONB NOT NULL DEFAULT '[]',
    fuse_directory_mounts JSONB NOT NULL DEFAULT '[]',
    -- Which vault key each secret the template declares is: the
    -- secret's name to the key, as the create or the last edit gave
    -- them, in that order.
    secrets               JSONB NOT NULL DEFAULT '{}',
    -- The provider it last ran on, and when it last began or ceased.
    last_provider         JSONB,
    last_active           TIMESTAMPTZ,
    tags                  TEXT[] NOT NULL DEFAULT '{}',
    created               TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator               JSONB NOT NULL,
    UNIQUE (template, index)
);

-- A tool: made from a template, or joined to another daemon's tool by
-- the daemon's record, by name, and the tool as that daemon names it.
CREATE TABLE IF NOT EXISTS diverge.tools (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- `created` or `connected`.
    kind                  TEXT NOT NULL,
    template              TEXT,
    provider              JSONB,
    connected_daemon      TEXT,
    connected_tool        JSONB,
    index                 BIGINT NOT NULL,
    name                  TEXT UNIQUE,
    fuse_file_mounts      JSONB NOT NULL DEFAULT '[]',
    fuse_directory_mounts JSONB NOT NULL DEFAULT '[]',
    -- Which vault key each secret the template declares is, as for
    -- an agent; empty for a connected tool.
    secrets               JSONB NOT NULL DEFAULT '{}',
    -- What its container declared it may ask of the daemon at its
    -- registration when it last started; empty for none.
    permissions           JSONB NOT NULL DEFAULT '[]',
    last_provider         JSONB,
    last_active           TIMESTAMPTZ,
    tags                  TEXT[] NOT NULL DEFAULT '{}',
    created               TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator               JSONB NOT NULL,
    CONSTRAINT tools_origin_daemon CHECK (
        (kind = 'created' AND template IS NOT NULL
            AND connected_daemon IS NULL AND connected_tool IS NULL)
        OR (kind = 'connected' AND template IS NULL AND provider IS NULL
            AND connected_daemon IS NOT NULL
            AND connected_tool IS NOT NULL)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS tools_created_index
    ON diverge.tools (template, index) WHERE kind = 'created';
-- A connected tool's once-and-for-all index counts among tools joined
-- to one daemon's one tool.
CREATE UNIQUE INDEX IF NOT EXISTS tools_connected_index
    ON diverge.tools (connected_daemon, connected_tool, index) WHERE kind = 'connected';

-- A tool attached to an agent; `seq` is the order of attaching.
CREATE TABLE IF NOT EXISTS diverge.attachments (
    tool        BIGINT NOT NULL REFERENCES diverge.tools(id) ON DELETE CASCADE,
    agent       BIGINT NOT NULL REFERENCES diverge.agents(id) ON DELETE CASCADE,
    seq         BIGINT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (tool, agent)
);
CREATE INDEX IF NOT EXISTS attachments_agent ON diverge.attachments (agent);

-- A tool attached to every agent of an agent template; `seq` is the
-- order of attaching. A template's row stays through its delete, so
-- the reference holds; the delete clears these rows itself.
CREATE TABLE IF NOT EXISTS diverge.template_attachments (
    tool        BIGINT NOT NULL REFERENCES diverge.tools(id) ON DELETE CASCADE,
    template    TEXT NOT NULL REFERENCES diverge.agents_templates(id),
    seq         BIGINT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (tool, template)
);
CREATE INDEX IF NOT EXISTS template_attachments_template ON diverge.template_attachments (template);

-- A dependency tool: a dependency template under one owner, as the
-- template's scope decides — a transient template or an `instance`
-- service one per agent, a `template` service one per agent template,
-- a `global` service one for all — made by the first registration
-- that declares the template under that owner. An instance tool goes
-- with its agent; a per-template tool with its agent template, by the
-- template's delete; a global tool with nothing but the dependency
-- template's delete, which takes every tool of the template. The
-- container is work made from the row, never the row.
CREATE TABLE IF NOT EXISTS diverge.dependency_tools (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    template       TEXT NOT NULL,
    -- Its number among the dependency tools ever made from the
    -- template, whatever their owners.
    index          BIGINT NOT NULL,
    -- `instance`, `template` or `global`.
    scope          TEXT NOT NULL,
    owner_agent    BIGINT REFERENCES diverge.agents(id) ON DELETE CASCADE,
    owner_template TEXT,
    tags           TEXT[] NOT NULL DEFAULT '{}',
    -- What its container declared it may ask of the daemon at its
    -- registration when it last started; empty for none.
    permissions    JSONB NOT NULL DEFAULT '[]',
    created        TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator        JSONB NOT NULL,
    CONSTRAINT dependency_tools_scope_owner CHECK (
        (scope = 'instance' AND owner_agent IS NOT NULL AND owner_template IS NULL)
        OR (scope = 'template' AND owner_agent IS NULL AND owner_template IS NOT NULL)
        OR (scope = 'global' AND owner_agent IS NULL AND owner_template IS NULL)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS dependency_tools_per_agent
    ON diverge.dependency_tools (template, owner_agent) WHERE scope = 'instance';
CREATE UNIQUE INDEX IF NOT EXISTS dependency_tools_per_template
    ON diverge.dependency_tools (template, owner_template) WHERE scope = 'template';
CREATE UNIQUE INDEX IF NOT EXISTS dependency_tools_global
    ON diverge.dependency_tools (template) WHERE scope = 'global';
CREATE INDEX IF NOT EXISTS dependency_tools_template ON diverge.dependency_tools (template);
CREATE UNIQUE INDEX IF NOT EXISTS dependency_tools_index ON diverge.dependency_tools (template, index);

-- An agent attached to a dependency tool: its last registration
-- declared the tool's template, at `declared`. `seq` is the order of
-- first attaching; a registration that declares it again moves
-- `declared` forward and nothing else.
CREATE TABLE IF NOT EXISTS diverge.dependency_attachments (
    tool        BIGINT NOT NULL REFERENCES diverge.dependency_tools(id) ON DELETE CASCADE,
    agent       BIGINT NOT NULL REFERENCES diverge.agents(id) ON DELETE CASCADE,
    seq         BIGINT GENERATED ALWAYS AS IDENTITY,
    declared    TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tool, agent)
);
CREATE INDEX IF NOT EXISTS dependency_attachments_agent ON diverge.dependency_attachments (agent);

-- The tags on a volume: the one thing the daemon keeps of one, which
-- is its provider's and listed by it. One row per tagged volume; a
-- volume whose tags are all taken off has none.
CREATE TABLE IF NOT EXISTS diverge.volume_tags (
    provider    JSONB NOT NULL,
    name        TEXT NOT NULL,
    tags        TEXT[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (provider, name)
);

-- What the daemon has learnt of an image's /DIVERGE.md, by digest: a
-- digest is the image, so an answer found once is the answer for good.
-- `document` is the document parsed; NULL is that the image ships none.
CREATE TABLE IF NOT EXISTS diverge.image_info (
    digest   TEXT PRIMARY KEY,
    document JSONB
);

-- A dependency template, by the hash of its own JSON: the body every
-- dependency tool of it shares. Made by the first registration that
-- declares it, gone with the template's delete.
CREATE TABLE IF NOT EXISTS diverge.dependencies (
    id       TEXT PRIMARY KEY,
    template JSONB NOT NULL
);

-- The graph of who sent to whom through the daemon: one row per
-- sender and receiver, with a count raised at every occurrence since
-- the first, and never timed: the id orders the edges by when each
-- was added. Each end is its kind and the columns that kind uses: an
-- account by its identity, compared by value since an administrator
-- has no account row, its edges deleted by the account's delete; an
-- agent, a standalone tool or a dependency tool by its row, so its
-- delete — or the delete of what it went with — takes its edges.
CREATE TABLE IF NOT EXISTS diverge.events (
    id                  BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- `account`, `agent`, `standalone_tool` or `dependency_tool`.
    sender_kind         TEXT NOT NULL,
    sender_issuer       TEXT,
    sender_subject      TEXT,
    sender_agent        BIGINT REFERENCES diverge.agents(id) ON DELETE CASCADE,
    sender_tool         BIGINT REFERENCES diverge.tools(id) ON DELETE CASCADE,
    sender_dependency   BIGINT REFERENCES diverge.dependency_tools(id) ON DELETE CASCADE,
    -- `account`, `agent`, `standalone_tool` or `dependency_tool`.
    receiver_kind       TEXT NOT NULL,
    receiver_issuer     TEXT,
    receiver_subject    TEXT,
    receiver_agent      BIGINT REFERENCES diverge.agents(id) ON DELETE CASCADE,
    receiver_tool       BIGINT REFERENCES diverge.tools(id) ON DELETE CASCADE,
    receiver_dependency BIGINT REFERENCES diverge.dependency_tools(id) ON DELETE CASCADE,
    count               BIGINT NOT NULL DEFAULT 1,
    CONSTRAINT events_sender CHECK (
        (sender_kind = 'account' AND sender_issuer IS NOT NULL AND sender_subject IS NOT NULL
            AND sender_agent IS NULL AND sender_tool IS NULL AND sender_dependency IS NULL)
        OR (sender_kind = 'agent' AND sender_agent IS NOT NULL AND sender_issuer IS NULL
            AND sender_subject IS NULL AND sender_tool IS NULL AND sender_dependency IS NULL)
        OR (sender_kind = 'standalone_tool' AND sender_tool IS NOT NULL AND sender_issuer IS NULL
            AND sender_subject IS NULL AND sender_agent IS NULL AND sender_dependency IS NULL)
        OR (sender_kind = 'dependency_tool' AND sender_dependency IS NOT NULL AND sender_issuer IS NULL
            AND sender_subject IS NULL AND sender_agent IS NULL AND sender_tool IS NULL)
    ),
    CONSTRAINT events_receiver CHECK (
        (receiver_kind = 'account' AND receiver_issuer IS NOT NULL AND receiver_subject IS NOT NULL
            AND receiver_agent IS NULL AND receiver_tool IS NULL AND receiver_dependency IS NULL)
        OR (receiver_kind = 'agent' AND receiver_agent IS NOT NULL AND receiver_issuer IS NULL
            AND receiver_subject IS NULL AND receiver_tool IS NULL AND receiver_dependency IS NULL)
        OR (receiver_kind = 'standalone_tool' AND receiver_tool IS NOT NULL AND receiver_issuer IS NULL
            AND receiver_subject IS NULL AND receiver_agent IS NULL AND receiver_dependency IS NULL)
        OR (receiver_kind = 'dependency_tool' AND receiver_dependency IS NOT NULL AND receiver_issuer IS NULL
            AND receiver_subject IS NULL AND receiver_agent IS NULL AND receiver_tool IS NULL)
    ),
    CONSTRAINT events_edge UNIQUE NULLS NOT DISTINCT (sender_kind, sender_issuer, sender_subject, sender_agent, sender_tool, sender_dependency,
        receiver_kind, receiver_issuer, receiver_subject, receiver_agent, receiver_tool, receiver_dependency)
);
CREATE INDEX IF NOT EXISTS events_sender_agent ON diverge.events (sender_agent);
CREATE INDEX IF NOT EXISTS events_receiver_agent ON diverge.events (receiver_agent);
CREATE INDEX IF NOT EXISTS events_sender_tool ON diverge.events (sender_tool);
CREATE INDEX IF NOT EXISTS events_receiver_tool ON diverge.events (receiver_tool);
CREATE INDEX IF NOT EXISTS events_sender_dependency ON diverge.events (sender_dependency);
CREATE INDEX IF NOT EXISTS events_receiver_dependency ON diverge.events (receiver_dependency);
