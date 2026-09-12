//! What an `authorize_hook` of [`Fixed`](super::Fixed) receives, and
//! what it answers.
//!
//! The hook is run as [`hook`](crate::hook) provides: one line of
//! JSON on stdin, the [`Input`]; exit `0` and one JSON document on
//! stdout, the [`Output`]. Anything else is a
//! [`hook::Error`](crate::hook::Error): the hook has not answered, and
//! the volume is not listed to that identity.
//!
//! ```json
//! {"identity": "acme", "volume": "datasets"}
//! ```
//!
//! ```json
//! {"authorized": true}
//! ```

mod input;
mod output;

pub use input::*;
pub use output::*;
