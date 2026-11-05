//! The identities a person signs in as.
//!
//! An identity is a subject — the `sub` of every token issued to it —
//! and the argon2id hash of its password. It is added, its password
//! changed and it is removed by the CLI, on the issuer's own host; the
//! issuer reads it at every sign-in. Removing one removes every code
//! and refresh token issued to it; changing its password revokes
//! every refresh token issued to it.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod add;
mod identities;
mod identity;
mod list;
mod password;
mod password_hash;
mod remove;

pub use identities::*;
pub use identity::*;
