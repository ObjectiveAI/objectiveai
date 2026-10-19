//! The dependency template bodies: what every dependency tool of a
//! template shares, by the template's hash.
//!
//! A template is the body alone: its tags, when it was made and who
//! made it are each [tool](crate::store::dependency_tools)'s. [`create`]
//! puts a body on record, or finds it there; [`by_id`] loads one;
//! [`delete`] removes one, which the template's delete does after its
//! tools.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod create;
mod delete;
mod load;

pub use create::*;
pub use delete::*;
pub use load::*;
