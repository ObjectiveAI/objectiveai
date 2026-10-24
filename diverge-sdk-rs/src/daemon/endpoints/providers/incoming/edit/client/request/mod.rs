//! Edit request data.
//!
//! What a caller hands the daemon to replace a credential: which one,
//! and the [`Patch`](crate::daemon::patch::Patch) to its [`Editable`]
//! member, the credential without a key.

mod editable;
mod frame;

pub use editable::*;
pub use frame::*;
