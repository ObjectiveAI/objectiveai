//! The authorization codes issued and not yet spent.
//!
//! A code is issued when a person signs in on the authorization page,
//! kept as its SHA-256 with everything its token request will need,
//! and spent once: [`take`](Codes::take) removes it as it reads it,
//! so a code presented twice is unknown the second time. An expired
//! code is removed at the next code's issue.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod code;
mod codes;
mod take;

pub use code::*;
pub use codes::*;
