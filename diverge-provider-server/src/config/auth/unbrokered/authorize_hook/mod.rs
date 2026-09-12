//! What an `authorize_hook` of [`Hook`](super::Hook) receives, and what
//! it answers.
//!
//! The hook is run as [`hook`](crate::hook) provides: one line of
//! JSON on stdin, the [`Input`]; exit `0` and one JSON document on
//! stdout, the [`Output`]. Anything else is a
//! [`hook::Error`](crate::hook::Error): the hook has not answered, and
//! the credential is refused.
//!
//! ```json
//! {"credential": "5f1c…", "address": "203.0.113.7"}
//! ```
//!
//! ```json
//! {"identity": "acme"}
//! ```
//!
//! ```json
//! {"refused": "no such key"}
//! ```

mod input;
mod output;

pub use input::*;
pub use output::*;
