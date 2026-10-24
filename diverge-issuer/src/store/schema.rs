//! The database's tables.

/// Every table, run on every open; there are no migrations.
///
/// - Every time is whole seconds since the Unix epoch, as
///   [`now`](crate::now) reads it.
/// - Every secret is a hash: a password as an argon2id PHC string, and
///   a device code or a refresh token as the lowercase hex SHA-256 of
///   the value. The signing keys are the one thing kept whole: a key's
///   PEM, which the issuer signs with. A user code is the one value kept as it
///   is, since a person types it.
/// - A scope is the space-separated list OAuth sends.
/// - Removing an identity removes, by cascade, every device code and
///   every refresh token issued to it.
/// - There is no client column: the issuer has one app,
///   [`CLIENT_ID`](crate::endpoints::CLIENT_ID).
pub(super) const SCHEMA: &str = "
create table if not exists identities (
    subject       text primary key,
    password_hash text not null,
    made          integer not null
);
create table if not exists keys (
    kid     text primary key,
    pem     text not null,
    made    integer not null
);
create table if not exists device_codes (
    hash      text primary key,
    user_code text not null unique,
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
