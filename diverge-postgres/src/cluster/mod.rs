//! The cluster: its superuser's password, and its one initialization.
//!
//! [`password`] reads `<dir>/password` or mints it — thirty-two hex
//! digits, written once, read at every start, never changed by this
//! program. [`init`] runs `initdb` into `<dir>/data` if `<dir>/data.ready`
//! is absent, with that password as the superuser's and
//! `scram-sha-256` as the authentication every connection makes, and
//! writes the marker; the marker is beside the data directory and
//! not in it, since `initdb` refuses a directory that is not empty.
//! A data directory without its marker is an initialization that did
//! not finish, and is thrown away as a partial extraction is.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod error;
mod init;
mod password;

pub use error::*;
pub use init::*;
pub use password::*;
