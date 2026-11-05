//! The host's identities: the credentials signed in under the root's
//! `identities/`, each one subject at one issuer, listed.
//!
//! [`issuers`] lists the issuers an identity is signed in at;
//! [`subjects`] lists the subjects signed in at one of them; [`login`]
//! signs one in, the one write. The rules:
//!
//! 1. **The host's files.** What is listed is the identities on the
//!    daemon's host, as `diverge identities login` signs them in: every
//!    one, whether or not the daemon presents it anywhere, whether or
//!    not it is signed out.
//! 2. **No token.** Nothing of an identity is read but its name, and no
//!    token is ever answered.
//! 3. **A snapshot.** Each listing is one answer and the finish: the
//!    files are not watched, so nothing is kept open.
//!
//! Split by who SENDS, as everywhere else. A client asks — so the
//! question is in each endpoint's `client` — and the daemon answers, so
//! the answer is in its `server`.

pub mod issuers;
pub mod login;
pub mod subjects;
