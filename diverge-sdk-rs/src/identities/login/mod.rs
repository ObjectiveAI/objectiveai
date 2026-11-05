//! Signing an identity in: OAuth's device authorization grant, or its
//! password grant.
//!
//! [`login_code`](super::Identities::login_code) is OpenID Connect's
//! own: the browser sent to the issuer, and its redirect to a loopback
//! port taken — see the [rules](super), rule 13.
//! [`login_password`](super::Identities::login_password) sends the
//! subject and its password and writes the identity from the one
//! answer — see the [rules](super), rule 12.
//! [`login`](super::Identities::login) reads the issuer's discovery
//! document and asks it for a device code; the [`Pending`] it answers
//! carries the user code a person enters at the verification URI, and
//! [`Pending::wait`] polls until the person has decided — see the
//! [rules](super), rule 6.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod code;
mod device;
mod password;
mod pending;

pub use code::*;
pub use pending::*;
