//! The outgoing provider records.
//!
//! [`Outgoing`] is one as the store holds it — the mode whole, with
//! its credential, which the wire never answers. [`all`] and
//! [`by_address`] load; [`create`], [`update_mode`],
//! [`set_last_connected`], [`set_tags`] and [`delete`] write, each
//! one statement, inside whatever transaction the caller holds.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod connected;
mod create;
mod delete;
mod edit;
mod load;
mod outgoing;
mod row;
mod tags;

pub use connected::*;
pub use create::*;
pub use delete::*;
pub use edit::*;
pub use load::*;
pub use outgoing::*;
pub use tags::*;
