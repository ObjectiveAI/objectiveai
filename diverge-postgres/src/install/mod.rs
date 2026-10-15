//! The Postgres binaries: extracted once into `bin/`, found at every
//! start.
//!
//! The archive baked into this program is one exact version of
//! Postgres, and it is extracted to `bin/<version>/`, under which
//! `bin/postgres`, `bin/pg_ctl` and `bin/initdb` are what the rest of
//! the program runs. [`ensure`] extracts it if `bin/<version>/complete`
//! is absent, under the lock `bin/locks/install.lock` so two starts
//! racing each other extract once, and answers the [`Binaries`] either
//! way; [`Binaries`] names the three programs. Every lock file this
//! program ever takes lives in `bin/locks/`, beside the binaries,
//! which is why `bin/` is made before anything else is.
//!
//! # A partial extraction is thrown away
//!
//! An extraction that did not reach its marker — the program was
//! killed, the disk filled — leaves a `bin/<version>/` nobody can
//! trust. The next start throws it away with [`discard`] and extracts
//! afresh; [`discard`] is how the cluster's partial initialization is
//! thrown away too.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod binaries;
mod discard;
mod ensure;
mod error;

pub use binaries::*;
pub use discard::*;
pub use ensure::*;
pub use error::*;
