//! The requests over routes, served: [`set`], [`delete`], [`list`].
//! Whether a container is served through a route now is [`in_use`],
//! false until containers run.

mod in_use;

pub use in_use::*;

pub mod delete;
pub mod list;
pub mod set;
