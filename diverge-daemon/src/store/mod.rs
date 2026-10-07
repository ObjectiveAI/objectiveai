//! The records: what the daemon holds, in the database it serves.
//!
//! Every durable thing the daemon knows — its accounts, their roles,
//! the grants of those roles, who made what and when — is a row in
//! the schema `diverge` of the one Postgres the daemon runs on, the
//! same one its containers are served from. [`Store`] is the pool and
//! the one way to begin a transaction; [`apply`] is the DDL, applied
//! at every start and harmless when it has been; [`seed`] is the one
//! role and the one account seeded into a database that had no
//! `accounts` table before, so that somebody can be the first to
//! connect — and both are ordinary records from then on, editable and
//! deletable. [`accounts`], [`roles`], [`providers_outgoing`] and
//! [`providers_incoming`] are the records of each kind, loaded whole
//! and written whole; [`of_account`] is what an account may
//! do, read fresh for every request; [`tags`] is the one way a set of
//! tags is kept; the ids are [`AccountId`] and [`RoleId`], so that an
//! id of one kind is never handed to a query of the other.
//!
//! # Whole or not at all
//!
//! Every request that writes runs inside one transaction that is
//! committed only on the answer that says it was done; dropping the
//! transaction on any other path is the rollback, which is how
//! "nothing changed" is kept without a word of code per answer. The
//! database's own constraints — one account per name, one credential
//! per identity, one role per name, a role held by no account before
//! it is dropped — are the answers `Exists`, `InUse` and the like, read
//! back from the violation rather than checked ahead of it, so that
//! two requests racing each other cannot both win.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod grants;
mod id;
mod root;
mod schema;
mod store;

pub use error::*;
pub use grants::*;
pub use id::*;
pub use root::*;
pub use schema::*;
pub use store::*;

pub mod accounts;
pub mod providers_incoming;
pub mod providers_outgoing;
pub mod roles;
pub mod tags;
