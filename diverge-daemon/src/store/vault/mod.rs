//! The vault records: keys, each holding one or more JSON values in
//! numbered slots.
//!
//! [`Entry`] is one as the store holds it — its [`Slot`]s, each with
//! the holder locking it, and the agents and tools binding it beside;
//! [`all`] and [`by_key`] load; [`insert`], [`set_tags`] and [`delete`]
//! write, each inside whatever transaction the caller holds. A run's
//! secrets go through four more, each in the caller's transaction:
//! [`acquire`] takes one free slot of a key for a holder — a random
//! one, skipping the slots other transactions are taking now —
//! [`read_random`] reads one without taking it, [`write_back`] puts a
//! holder's value into the slot it holds and frees it, [`release`]
//! frees it unchanged; and [`clear_holders`] frees every slot at boot,
//! since no run survives a restart. A key some record binds is not
//! deleted: the binding lives inside the record's `secrets`, which no
//! constraint reaches, so [`delete`] asks [`in_use`](crate::store::in_use)
//! first and answers as a value; nor is a slot a run holds, nor a
//! key's last slot, which the same value says.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod acquire;
mod clear;
mod delete;
mod entry;
mod load;
mod read;
mod release;
mod row;
mod insert;
mod tags;
mod write_back;

pub use acquire::*;
pub use clear::*;
pub use delete::*;
pub use entry::*;
pub use load::*;
pub use read::*;
pub use release::*;
pub use insert::*;
pub use tags::*;
pub use write_back::*;
