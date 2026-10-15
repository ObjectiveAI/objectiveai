//! The daemon records: other daemons the caller holds an account on.
//!
//! [`Record`] is one as the store holds it — the links, and its
//! accounts whole, each mode with its credential, which the wire never
//! answers. [`all`] and [`by_name`] load, accounts and all;
//! [`create`], [`update`], [`set_tags`] and [`delete`] write the record,
//! and [`accounts`] its accounts, each one statement, inside whatever
//! transaction the caller holds.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod edit;
mod load;
mod record;
mod row;
mod tags;

pub mod accounts;

pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use record::*;
pub use tags::*;
