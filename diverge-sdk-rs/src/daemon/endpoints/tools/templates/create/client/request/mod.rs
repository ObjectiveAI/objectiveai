//! Create request data.
//!
//! What a caller hands the daemon to make a template: the
//! [`Template`](crate::daemon::endpoints::tools::templates::Template) itself, and nothing else — no
//! name, since the template's id is its hash.

mod frame;

pub use frame::*;
