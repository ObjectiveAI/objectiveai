//! The account records.
//!
//! [`Account`] is one as the store holds it — more than the wire
//! reports, since the key hash and the row id are here and are never
//! sent. [`all`], [`by_reference`], [`by_id`] and [`by_key_hash`] load;
//! [`create`], [`update`], [`set_roles`], [`set_tags`] and [`delete`]
//! write, each one statement, inside whatever transaction the caller
//! holds. A write answers the database's own refusal — a name or an
//! identity already taken — as a value rather than an error, since
//! that refusal is an answer on the wire.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod create;
mod delete;
mod edit;
mod load;
mod roles;
mod row;
mod tags;

pub use account::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use roles::*;
pub use tags::*;
