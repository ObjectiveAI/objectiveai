//! The one `config.yaml` every Diverge program reads.
//!
//! - `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge/`,
//!   names the ROOT, created if absent — see [`root`]. The root is
//!   one directory for every Diverge program on the host: the
//!   provider keeps its state under `<root>/provider/`, the daemon
//!   under `<root>/daemon/`, and the daemon's Postgres under
//!   `<root>/daemon/postgres/`.
//! - `<root>/config.yaml` is read if present; absent means the
//!   built-in defaults, [`Config::default`]. No other name or
//!   extension is looked for — see [`load`].
//! - One block per program, at the top: [`provider`], read by
//!   `diverge-provider`, and [`daemon`], read by `diverge-daemon`
//!   and, for its `postgres`, by `diverge-postgres`. A block that is
//!   absent, or a bare key, is that program's defaults; a key no
//!   block has is refused, so a misspelled setting is refused rather
//!   than ignored. Each program reads the whole file and takes its
//!   block, so a block that does not parse stops every program.
//! - Every path inside the file resolves relative to the root, the
//!   directory that holds the file, never to the working directory.
//!   [`load`] resolves them as it reads, so a program holds every
//!   path absolute.
//!
//! [`Config`] is the document; [`Error`] is why the root or the file
//! could not be used. What the file names on disk — a store to make,
//! a directory that must exist — is each program's to check at its
//! own start: this module reads the document, and nothing else.
//!
//! ```yaml
//! provider:
//!   port: 14979
//!   containers:
//!     podman:
//!       registries:
//!         - host: docker.io
//!       storage_path: provider/podman_data
//!       image_cache_disk: 34359738368
//!       container_overlay_disk: 34359738368
//!       memory: 8589934592
//! daemon:
//!   port: 14980
//!   postgres:
//!     kind: local
//!     max_connections: 1024
//!   idle_seconds: 10
//! ```
//!
//! # Why the SDK
//!
//! Three programs read one file, and one shape, in one place, is how
//! they agree on it: the provider's block is the provider's and the
//! daemon's is the daemon's, but the file, the root and the rule for
//! a path are everyone's. Nothing here crosses a wire; it is in this
//! crate because it is shared, as [`file_lock`](crate::file_lock) is.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod error;
mod load;
mod root;
pub mod daemon;
pub mod provider;

pub use config::*;
pub use error::*;
pub use load::*;
pub use root::*;
