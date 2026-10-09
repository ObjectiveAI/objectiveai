-- The daemon's own schema, in the database it serves. Every statement
-- is idempotent, so this runs at every start.

CREATE SCHEMA IF NOT EXISTS diverge;

-- Nothing that is not the daemon reads these tables: a container's
-- login role lives in this same database and must never see a key
-- hash.
REVOKE ALL ON SCHEMA diverge FROM PUBLIC;

CREATE TABLE IF NOT EXISTS diverge.accounts (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- One account per name.
    name        TEXT UNIQUE,
    -- One credential per identity; the key is kept as its SHA-256 hex.
    identity    TEXT UNIQUE,
    key_hash    TEXT UNIQUE,
    -- The one peer address the key is accepted from, as text; absent,
    -- any address.
    address     TEXT,
    description TEXT,
    -- Sorted bytewise, deduplicated, rewritten whole.
    tags        TEXT[] NOT NULL DEFAULT '{}',
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The wire's Creator, as JSON.
    creator     JSONB NOT NULL,
    -- A name, a credential, or both, and never neither.
    CONSTRAINT accounts_never_neither CHECK (name IS NOT NULL OR identity IS NOT NULL),
    -- A credential is an identity and a key together, and an address
    -- only beside them.
    CONSTRAINT accounts_credential_whole CHECK (
        (identity IS NULL) = (key_hash IS NULL) AND (address IS NULL OR identity IS NOT NULL)
    )
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

CREATE TABLE IF NOT EXISTS diverge.providers_outgoing (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- The address is the identity: the string as given, dialled as is.
    address        TEXT NOT NULL UNIQUE,
    -- The wire's Mode, as JSON: how the daemon authenticates there,
    -- credential and all. Never answered whole.
    mode           JSONB NOT NULL,
    -- When the daemon's connection to it last opened or closed; absent
    -- for one never dialled.
    last_connected TIMESTAMPTZ,
    created        TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator        JSONB NOT NULL
);

CREATE TABLE IF NOT EXISTS diverge.providers_incoming (
    id       BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- One credential per identity; the key is kept as its SHA-256 hex.
    identity TEXT NOT NULL UNIQUE,
    key_hash TEXT NOT NULL UNIQUE,
    -- The one peer address the key is accepted from, as text; absent,
    -- any address.
    address  TEXT,
    created  TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator  JSONB NOT NULL
);

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

-- An agent: a template plus what its create added. The container is
-- work made from the row, never the row.
CREATE TABLE IF NOT EXISTS diverge.agents (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    template              TEXT NOT NULL,
    index                 BIGINT NOT NULL,
    name                  TEXT UNIQUE,
    -- The account it runs under, if any; an account a container
    -- names is not deleted.
    account               BIGINT REFERENCES diverge.accounts(id) ON DELETE RESTRICT,
    -- The provider it is pinned to and that provider's volumes
    -- mounted, as the create named them.
    provider              JSONB,
    fuse_file_mounts      JSONB NOT NULL DEFAULT '[]',
    fuse_directory_mounts JSONB NOT NULL DEFAULT '[]',
    -- The deployer agent as it was when named: template, index, name.
    deployer              JSONB,
    -- The provider it last ran on, and when it last began or ceased.
    last_provider         JSONB,
    last_active           TIMESTAMPTZ,
    tags                  TEXT[] NOT NULL DEFAULT '{}',
    created               TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator               JSONB NOT NULL,
    UNIQUE (template, index)
);

-- A tool: made from a template, or joined to somebody else's
-- container by its provider, its id and an authorization.
CREATE TABLE IF NOT EXISTS diverge.tools (
    id                    BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- `created` or `connected`.
    kind                  TEXT NOT NULL,
    template              TEXT,
    provider              JSONB,
    connected_provider    JSONB,
    connected_id          TEXT,
    authorization         TEXT,
    index                 BIGINT NOT NULL,
    name                  TEXT UNIQUE,
    account               BIGINT REFERENCES diverge.accounts(id) ON DELETE RESTRICT,
    fuse_file_mounts      JSONB NOT NULL DEFAULT '[]',
    fuse_directory_mounts JSONB NOT NULL DEFAULT '[]',
    last_provider         JSONB,
    last_active           TIMESTAMPTZ,
    tags                  TEXT[] NOT NULL DEFAULT '{}',
    created               TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator               JSONB NOT NULL,
    CONSTRAINT tools_origin_whole CHECK (
        (kind = 'created' AND template IS NOT NULL
            AND connected_provider IS NULL AND connected_id IS NULL AND authorization IS NULL)
        OR (kind = 'connected' AND template IS NULL AND provider IS NULL
            AND connected_provider IS NOT NULL AND connected_id IS NOT NULL AND authorization IS NOT NULL)
    )
);
CREATE UNIQUE INDEX IF NOT EXISTS tools_created_index
    ON diverge.tools (template, index) WHERE kind = 'created';
CREATE UNIQUE INDEX IF NOT EXISTS tools_connected_index
    ON diverge.tools (connected_provider, connected_id, index) WHERE kind = 'connected';

-- A tool attached to an agent; `seq` is the order of attaching.
CREATE TABLE IF NOT EXISTS diverge.attachments (
    tool        BIGINT NOT NULL REFERENCES diverge.tools(id) ON DELETE CASCADE,
    agent       BIGINT NOT NULL REFERENCES diverge.agents(id) ON DELETE CASCADE,
    seq         BIGINT GENERATED ALWAYS AS IDENTITY,
    PRIMARY KEY (tool, agent)
);
CREATE INDEX IF NOT EXISTS attachments_agent ON diverge.attachments (agent);

-- Who may see a created tool from its provider, and who may join it:
-- the key, when the admission admits a connect, is kept as its hash.
CREATE TABLE IF NOT EXISTS diverge.admissions (
    tool        BIGINT NOT NULL REFERENCES diverge.tools(id) ON DELETE CASCADE,
    identity    TEXT NOT NULL,
    address     TEXT,
    -- `list`, `connect` or `both`.
    admits      TEXT NOT NULL,
    key_hash    TEXT UNIQUE,
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tool, identity)
);

-- A route: at one position — an agent, by name, and the template of
-- a dependency it declares — this tool answers, and no deployer is
-- asked. The tool gone takes the route.
CREATE TABLE IF NOT EXISTS diverge.routes (
    agent       TEXT NOT NULL,
    template    TEXT NOT NULL,
    tool        BIGINT NOT NULL REFERENCES diverge.tools(id) ON DELETE CASCADE,
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator     JSONB NOT NULL,
    PRIMARY KEY (agent, template)
);
