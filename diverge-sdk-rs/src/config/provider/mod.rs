//! The `provider` block of `config.yaml`: what `diverge-provider` is
//! told.
//!
//! [`Config`] is the block; [`auth`], [`clients`], [`containers`] and
//! [`volumes`] its sections. The provider keeps its state under
//! `<root>/provider/`: `hooks/`, where every folder is a hook the
//! block names by the folder's name alone; `run/`, its own scratch;
//! and podman's data under `containers.podman.storage_path`, which is
//! `<root>/provider/podman_data/` unless the block says otherwise.
//! What the block names on disk — a store to make, a fixed volume
//! that must exist — the provider checks at its start.
//!
//! ```yaml
//! provider:
//!   port: 14979
//!   auth:
//!     unbrokered:
//!       - key: 5f1c…
//!         identity: acme
//!         address: 203.0.113.7
//!       - key: 9a0e…
//!         identity: bolt
//!       - authorize_hook: peers
//!   clients:
//!     unbrokered:
//!       - address: hub.example.com:7000
//!         key: c31d…
//!         identity: hub
//!   containers:
//!     podman:
//!       registries:
//!         - host: docker.io
//!         - host: ghcr.io
//!           credential:
//!             username: bolt
//!             password: ghp_…
//!       storage_path: /mnt/podman
//!       image_cache_disk: 107374182400
//!       container_overlay_disk: 214748364800
//!       memory: 34359738368
//!   volumes:
//!     stores:
//!       - path: /mnt/volumes-a
//!         capacity: 1099511627776
//!     fixed:
//!       - name: datasets
//!         path: /srv/datasets
//!         bytes: 536870912000
//!         mode: ephemeral
//!         authorize_hook: datasets
//! ```
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod config;
pub mod auth;
pub mod clients;
pub mod containers;
pub mod volumes;

pub use config::*;
