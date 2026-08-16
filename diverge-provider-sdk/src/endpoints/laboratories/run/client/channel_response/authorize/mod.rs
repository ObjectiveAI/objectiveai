//! The answer to an authorization request.
//!
//! Yes or no, and if yes, the nickname the runner wants that connector
//! known by — which is what comes back when it leaves. See [`Frame`].

mod frame;

pub use frame::*;
