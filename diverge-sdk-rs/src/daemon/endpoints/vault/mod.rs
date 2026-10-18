//! The vault: keys the daemon keeps, each holding one or more JSON
//! values, for the secrets a container is handed.
//!
//! A VAULT KEY is a name and its values, each any JSON, numbered from
//! `0` in the order they were appended: its SLOTS. A template
//! declares the secrets it needs by name; an agent or a tool made
//! from it binds each name to a vault key; and its program, at
//! registration, says which names it takes and in what mode — read,
//! or held and written back. At every activation of the container
//! the daemon hands it one value per name, beside the arguments: for
//! a `fixed` name a random slot's value, read and never locked; for
//! a `rotating` name a free slot's value, LOCKED to the record —
//! the agent, the standalone tool, or the dependency tool — until
//! the container goes inactive, when the daemon writes back what the
//! program made of it and drops the lock. A lock is on a value,
//! never on a key: a key with several values serves several
//! containers at once, each holding one, and a key with every value
//! held makes the next activation fail. The container never asks the
//! daemon for a secret, and is never told a key. A key is inserted into,
//! read, listed by name and never by value, tagged and deleted here; a key
//! some agent or tool binds is not deleted, nor a value a running
//! container holds, which the delete says with a word of its own
//! each; a free value is replaced still — the next activation that takes
//! it is handed the new one. One key per name, and the name is what
//! every request names the key by; naming a key in a container's
//! `secrets` takes the `use` grant over it, and the key must be one
//! the vault holds, with at least one value.
//!
//! [`insert`] makes a key with its first value, appends a value, or
//! replaces one slot's value; [`get`] answers one key with every
//! value and who holds each; [`list`] lists them by name, narrowed,
//! each with its slots and their locks, its tags and the agents and
//! tools binding it, and never a value; [`delete`] removes one key
//! nothing binds, or one free value of a key that has another;
//! [`tag`] and [`untag`] change its tags. What a list reports is a
//! [`Key`](list::server::response::Key); what a get answers is an
//! [`Entry`](get::server::response::Entry), the key with its slots.

pub mod delete;
pub mod get;
pub mod list;
pub mod insert;
pub mod tag;
pub mod untag;
