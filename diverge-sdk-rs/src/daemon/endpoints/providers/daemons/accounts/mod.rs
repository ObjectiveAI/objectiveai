//! The accounts on a daemon record: what this daemon holds on another
//! daemon, each by a name of the caller's choosing and the mode it
//! authenticates there in.
//!
//! A daemon record is one daemon; its accounts are zero or more, each
//! a name and a mode, added, rotated and forgotten one at a time —
//! [`add`], [`edit`], [`delete`] — and reported by name and kind,
//! never by credential, on the record's list item. Whatever acts as
//! this daemon on the remote names the record and the account:
//! a connected tool's `connect`, a relayed request. This daemon holds
//! one connection per account at a time.

pub mod add;
pub mod delete;
pub mod edit;
