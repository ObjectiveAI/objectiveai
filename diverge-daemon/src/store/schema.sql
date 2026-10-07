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

-- A resource is held by the hash of its bytes, which live on disk
-- under <dir>/resources/<id>; the row is what is known about them.
CREATE TABLE IF NOT EXISTS diverge.resources (
    id          TEXT PRIMARY KEY,
    -- `file` or `directory`.
    kind        TEXT NOT NULL,
    description TEXT NOT NULL,
    bytes       BIGINT NOT NULL,
    tags        TEXT[] NOT NULL DEFAULT '{}',
    created     TIMESTAMPTZ NOT NULL DEFAULT now(),
    creator     JSONB NOT NULL,
    deleted     BOOLEAN NOT NULL DEFAULT false
);
