//! The Postgres binaries: extracted once into `bin/`, found at every
//! start.
//!
//! The archive baked into this program is one exact version of
//! Postgres, and it is extracted to `bin/<version>/`, under which
//! `bin/postgres`, `bin/pg_ctl` and `bin/initdb` are what the rest of
//! the program runs. [`ensure`] extracts it if `bin/<version>/complete`
//! is absent, or is in an extraction without the archive's hard
//! links, under the lock `bin/locks/install.lock` so two starts
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
//! # The archive's hard links are made here
//!
//! The archive names some of its files twice — every time zone that
//! is another's alias, `UTC` among them — the second name a hard
//! link, and the crate that extracts it passes those over. [`links`]
//! has every extraction make them, and [`linked`] tells an extraction
//! that has them from one made before they were: that one's marker
//! is not believed, and it is thrown away and extracted afresh as a
//! partial extraction is.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod binaries;
mod discard;
mod ensure;
mod error;
mod linked;
mod links;

pub use binaries::*;
pub use discard::*;
pub use ensure::*;
pub use error::*;
pub use linked::*;
pub use links::*;
