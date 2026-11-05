//! The authorization codes issued and not yet spent, RFC 6749 §4.1's.
//!
//! A code is issued at the authorization endpoint once the subject and
//! its password are right, and spent at the token endpoint. Its life, as
//! rules:
//!
//! 1. It is kept as its SHA-256, bound to the subject, the redirect URI,
//!    the PKCE challenge, the nonce, the scope, the audience and when
//!    the subject signed in — see [`issue`](Codes::issue).
//! 2. It is good for [`LIFETIME`] seconds from its issue.
//! 3. It is spent once: [`take`](Codes::take) reads and removes it in one
//!    transaction, so a second presentation finds nothing.
//! 4. An expired code is removed at the next code's issue, and is never
//!    taken.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod code;
mod codes;
mod issue;
mod take;

pub use code::*;
pub use codes::*;
