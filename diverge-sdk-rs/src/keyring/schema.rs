//! The keyring's one table.

/// Every entry: its name, its key, and when the key was made. Run on
/// every open; there are no migrations.
pub(super) const SCHEMA: &str = "
create table if not exists keyring_keys (
    name text primary key,
    key  text not null,
    made text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
)";
