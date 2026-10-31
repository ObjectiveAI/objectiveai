//! The account records.
//!
//! [`Account`] is one as the store holds it — more than the wire
//! reports, since the row id is here and is never sent. An account is
//! its identity, the `(issuer, subject)` a client's token establishes.
//! [`all`], [`by_identity`] and [`by_id`] load; [`create`], [`update`],
//! [`set_roles`], [`set_tags`] and [`delete`] write, each one
//! statement, inside whatever transaction the caller holds. A create
//! answers the database's own refusal — an identity already taken — as
//! a value rather than an error, since that refusal is an answer on
//! the wire.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod account;
mod activity;
mod create;
mod delete;
mod edit;
mod load;
mod roles;
mod row;
mod tags;

pub use account::*;
pub use activity::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use roles::*;
pub use tags::*;
