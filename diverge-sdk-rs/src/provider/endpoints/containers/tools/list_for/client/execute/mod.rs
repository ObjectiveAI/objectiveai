//! Performing a listing, rather than describing it.
//!
//! [`execute`] opens the scope naming the identity whose containers
//! are asked about, and hands back an [`ExecuteStream`] of what the
//! provider sends — containers added as their runners say yes, the
//! word that the listing is whole, then containers added and removed
//! as the identity's runs begin and end — with the [`Stop`] that ends
//! it. A listing has no end of its own, so the stop is how a lister
//! that has read what it wanted lets the scope go.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod execute;
mod execute_stream;
mod stop;

pub use error::*;
pub use execute::*;
pub use execute_stream::*;
pub use stop::*;
