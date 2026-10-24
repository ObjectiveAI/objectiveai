//! The database's tables.

/// Every table, run on every open; there are no migrations.
///
/// - Every time is whole seconds since the Unix epoch, as
///   [`now`](crate::now) reads it.
/// - Every secret is a hash: a password as an argon2id PHC string, and
///   a client secret, a code or a refresh token as the lowercase hex
///   SHA-256 of the value. A user code is the one value kept as it
///   is, since a person types it.
/// - A scope is the space-separated list OAuth sends.
/// - Removing an identity or a client removes, by cascade, every code
///   and every refresh token issued to it.
pub(super) const SCHEMA: &str = "
create table if not exists identities (
    subject       text primary key,
    password_hash text not null,
    made          integer not null
);
create table if not exists clients (
    id          text primary key,
    kind        text not null check (kind in ('public', 'confidential')),
    secret_hash text,
    made        integer not null,
    check ((kind = 'confidential') = (secret_hash is not null))
);
create table if not exists client_grants (
    client text not null references clients (id) on delete cascade,
    name   text not null check (name in ('authorization_code', 'device_code', 'refresh_token', 'client_credentials')),
    primary key (client, name)
);
create table if not exists client_audiences (
    client   text not null references clients (id) on delete cascade,
    position integer not null,
    audience text not null,
    primary key (client, position),
    unique (client, audience)
);
create table if not exists client_redirects (
    client text not null references clients (id) on delete cascade,
    uri    text not null,
    primary key (client, uri)
);
create table if not exists keys (
    kid     text primary key,
    entry   text not null unique,
    made    integer not null,
    retired integer
);
create table if not exists authorization_codes (
    hash         text primary key,
    client       text not null references clients (id) on delete cascade,
    subject      text not null references identities (subject) on delete cascade,
    redirect_uri text not null,
    challenge    text not null,
    scope        text not null,
    audience     text not null,
    nonce        text,
    auth_time    integer not null,
    expires      integer not null
);
create table if not exists device_codes (
    hash      text primary key,
    user_code text not null unique,
    client    text not null references clients (id) on delete cascade,
    scope     text not null,
    audience  text not null,
    expires   integer not null,
    interval  integer not null,
    last_poll integer,
    state     text not null check (state in ('pending', 'approved', 'denied')),
    subject   text references identities (subject) on delete cascade,
    auth_time integer,
    check ((state = 'approved') = (subject is not null)),
    check ((subject is null) = (auth_time is null))
);
create table if not exists refresh_chains (
    id        integer primary key,
    client    text not null references clients (id) on delete cascade,
    subject   text not null references identities (subject) on delete cascade,
    scope     text not null,
    audience  text not null,
    auth_time integer not null,
    made      integer not null,
    revoked   integer
);
create table if not exists refresh_tokens (
    hash  text primary key,
    chain integer not null references refresh_chains (id) on delete cascade,
    made  integer not null,
    used  integer
);
create index if not exists refresh_tokens_chain on refresh_tokens (chain);
";
