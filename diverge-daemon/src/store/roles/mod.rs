//! The role records.
//!
//! [`Role`] is one as the store holds it, with the accounts holding
//! it beside; [`all`], [`by_name`] and [`by_names`] load; [`create`],
//! [`update`], [`set_tags`] and [`delete`] write, each inside whatever
//! transaction the caller holds. A role held by an account is not
//! deleted, and the database says so by its own constraint, which
//! [`delete`] answers as a value.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod edit;
mod load;
mod role;
mod row;
mod tags;

pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use role::*;
pub use tags::*;
