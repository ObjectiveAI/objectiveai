//! The incoming credential records.
//!
//! [`Incoming`] is one as the store holds it — with the hash of its
//! key, which the wire never carries. [`all`], [`by_identity`] and
//! [`by_key_hash`] load; [`create`], [`update`] and [`delete`] write,
//! each one statement, inside whatever transaction the caller holds.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod edit;
mod incoming;
mod load;
mod row;

pub use create::*;
pub use delete::*;
pub use edit::*;
pub use incoming::*;
pub use load::*;
