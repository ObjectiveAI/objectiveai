//! One way of judging an unbrokered credential: a [`Key`] the
//! credential must equal, or a [`Hook`] that judges it. [`Unbrokered`]
//! is either.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod hook;
mod key;
mod unbrokered;

pub use hook::*;
pub use key::*;
pub use unbrokered::*;
