//! The host's identities: the credentials signed in under the root's
//! `identities/`, each one subject at one issuer, listed.
//!
//! [`list`] lists them, each with its tags; [`tag`] and [`untag`]
//! change an identity's tags, which the daemon keeps, since an identity
//! is a file and no record; [`login`] signs one in. The rules:
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

pub mod list;
pub mod login;
pub mod tag;
pub mod untag;
