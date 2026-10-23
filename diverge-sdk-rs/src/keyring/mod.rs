//! The keyring: every key a Diverge program holds, in one file that
//! every Diverge program on the host shares.
//!
//! A key is the credential a dialler presents on a connection's first
//! frame. The daemon, the provider, the CLI and the viewer each keep
//! theirs here — in `<root>/keyring.db`, a SQLite file read through
//! sqlx — and nowhere else. A configuration, a record and a request
//! name a key by its entry's name; none of them holds the key.
//!
//! # A key has no way out
//!
//! [`Key`] is the one type a key is held in. It has no `Serialize`, no
//! `Deserialize`, no `Display`, and no sqlx encoding, and its `Debug`
//! prints `Key(<hidden>)`; so a frame, a configuration, a database row
//! or a log line that would hold one does not compile. Its memory is
//! zeroed when it drops. [`Key::expose`] is the one way to its text,
//! and it is called only where a key is presented on a wire, compared
//! with one presented, or hashed.
//!
//! # The file
//!
//! The file is made readable and writable by its owner only, on the
//! first open that creates it (on Windows it keeps the user profile's
//! permissions, which are the user's alone). It is not encrypted: its
//! protection is the operating system's, as a private SSH key's is.
//! Several processes open it at once; it is kept in write-ahead-log
//! mode, and a write waits for another process's write to finish.
//!
//! # Rules
//!
//! 1. An entry's name is unique. [`mint`](Keyring::mint) and
//!    [`set`](Keyring::set) refuse a name already held; neither
//!    replaces a key.
//! 2. [`rotate`](Keyring::rotate) replaces the key of an entry that
//!    exists, and refuses a name that does not.
//! 3. [`list`](Keyring::list) answers names and when each was made,
//!    never a key.
//! 4. Nothing here prints, logs, or answers a key.
//!
//! Behind the `keyring` feature: sqlx and SQLite do not build where
//! most dependents do — the images inside containers — and nothing
//! inside a container holds a key.

mod entry;
mod failure;
mod get;
mod key;
mod keyring;
mod list;
mod mint;
mod remove;
mod rotate;
mod schema;
mod set;

pub use entry::*;
pub use failure::*;
pub use key::*;
pub use keyring::*;
