//! The vault: keys the daemon keeps, each holding one JSON value, for
//! the secrets a container is made with.
//!
//! A VAULT KEY is a name and a value, any JSON. A template declares
//! the secrets it needs by name; an agent or a tool made from it
//! binds each name to a vault key, and at the container's start the
//! daemon hands the container each value under its name, beside the
//! arguments — the container never asks the daemon for a secret, and
//! is never told a key. A key is set, read, listed by name and never
//! by value, tagged and deleted here; a key some agent or tool binds
//! is not deleted, which the delete says with a word of its own, and
//! is set still — the next start of that agent or tool is made with
//! the new value. One key per name, and the name is what every
//! request names the key by; naming a key in a container's `secrets`
//! takes the `use` grant over it.
//!
//! [`set`] makes a key or replaces its value; [`get`] answers one
//! with its value; [`list`] lists them by name, narrowed, each with
//! its tags and the agents and tools binding it, and never the value;
//! [`delete`] removes one nothing binds; [`tag`] and [`untag`] change
//! its tags. What a list reports is a
//! [`Key`](list::server::response::Key); what a get answers is an
//! [`Entry`](get::server::response::Entry), the key with its value.

pub mod delete;
pub mod get;
pub mod list;
pub mod set;
pub mod tag;
pub mod untag;
