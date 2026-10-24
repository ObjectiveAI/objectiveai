//! The one `provider.yaml` the provider reads.
//!
//! - `--config <dir>`, else `DIVERGE_CONFIG`, else `~/.diverge/`,
//!   names the ROOT, created if absent — see [`root`]. The root is
//!   one directory for every Diverge program on the host, each with
//!   its own document in it and its own directory under it: the
//!   provider keeps its state under `<root>/provider/`.
//! - `<root>/provider.yaml` is read if present; absent means the
//!   built-in defaults, [`Config::default`]. No other name or
//!   extension is looked for — see [`load`].
//! - [`Config`] is the document: its own `identities`, the `accepted`
//!   identities, and its sections [`clients`], [`containers`],
//!   [`volumes`] and [`images`]. A
//!   section that is absent is its defaults; a key the document does
//!   not have is refused, so a misspelled setting is refused rather
//!   than ignored.
//! - Every path inside the file resolves relative to the root, the
//!   directory that holds the file, never to the working directory.
//!   [`load`] resolves them as it reads, so the provider holds every
//!   path absolute. What the file names on disk — a store to make, a
//!   fixed volume that must exist — the provider checks at its start:
//!   this module reads the document, and nothing else.
//!
//! The provider keeps its state under `<root>/provider/`: `hooks/`,
//! where every folder is a hook the document names by the folder's
//! name alone; `run/`, its own scratch; and podman's data under
//! `containers.podman.storage_path`, which is
//! `<root>/provider/podman_data/` unless the document says otherwise.
//!
//! ```yaml
//! port: 14979
//! identities:
//!   https://auth.example.com: provider-1
//! accepted:
//!   - issuer: https://auth.example.com
//!     subject: daemon-1
//!   - issuer: https://auth.example.com
//!     subject: hub
//! clients:
//!   - address: hub.example.com:7000
//!     issuer: https://auth.example.com
//! containers:
//!   podman:
//!     registries:
//!       - host: docker.io
//!       - host: ghcr.io
//!         credential:
//!           username: bolt
//!           password: ghp_example
//!     storage_path: /mnt/podman
//!     image_cache_disk: 107374182400
//!     container_overlay_disk: 214748364800
//!     memory: 34359738368
//! volumes:
//!   stores:
//!     - path: /mnt/volumes-a
//!       capacity: 1099511627776
//!   fixed:
//!     - name: datasets
//!       path: /srv/datasets
//!       bytes: 536870912000
//!       mode: ephemeral
//!       authorize_hook: datasets
//! images:
//!   path: /mnt/images
//!   capacity: 274877906944
//! ```
//!
//! Every handshake, whichever end dialled, presents the token of the
//! provider's own identity at the peer's issuer — `identities` — and
//! serves the peer only when its verified `(issuer, subject)` is in
//! `accepted`. The tokens are the root's
//! [`identities`](diverge_sdk::identities), signed in beforehand.
//!
//! [`Error`] is why the root or the file could not be used.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
mod error;
mod load;
mod root;
pub mod clients;
pub mod containers;
pub mod images;
pub mod volumes;

pub use config::*;
pub use error::*;
pub use load::*;
pub use root::*;
