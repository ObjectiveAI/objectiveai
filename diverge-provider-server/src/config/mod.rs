//! The provider's configuration: where it lives, how it is found, and
//! what it says.
//!
//! - `--config <dir>`, else `DIVERGE_PROVIDER_CONFIG`, else
//!   `~/.diverge/provider/`, names the provider's DIRECTORY, created
//!   if absent.
//! - `<dir>/config.yaml` is read if present; absent means the built-in
//!   defaults, [`Config::default`], on which the provider runs
//!   containers and nothing else. No other name or extension is
//!   looked for.
//! - Every path inside `config.yaml` resolves relative to `<dir>`,
//!   never to the working directory.
//! - `<dir>/hooks/`, `<dir>/logs/`, `<dir>/data/` and `<dir>/run/` sit
//!   beside it; `data` is the one the file may move. Every folder
//!   under `hooks/` is a hook, and the file names one by that
//!   folder's name alone — see [`hook`](crate::hook).
//!
//! [`Config`] is the document; [`auth`], [`clients`], [`containers`]
//! and [`volumes`] its sections.
//!
//! ```yaml
//! auth:
//!   unbrokered:
//!     - key: 5f1c…
//!       identity: acme
//!       address: 203.0.113.7
//!     - key: 9a0e…
//!       identity: bolt
//!     - authorize_hook: peers
//! clients:
//!   unbrokered:
//!     - address: hub.example.com:7000
//!       key: c31d…
//!       identity: hub
//! containers:
//!   podman:
//!     registries:
//!       - host: docker.io
//!       - host: ghcr.io
//!         credential:
//!           username: bolt
//!           password: ghp_…
//!     storage_path: /mnt/podman
//!     image_cache_disk: 107374182400
//!     container_overlay_disk: 214748364800
//!     memory: 34359738368
//!   identity:
//!     storage_path: /mnt/identity
//!     disk: 107374182400
//! volumes:
//!   stores:
//!     - path: /mnt/volumes-a
//!       capacity: 1099511627776
//!   fixed:
//!     - name: datasets
//!       path: /srv/datasets
//!       authorize_hook: datasets
//! ```
//!
//! Finding the directory, reading the file, validating it, and running
//! a hook are not written yet.

mod config;
pub mod auth;
pub mod clients;
pub mod containers;
pub mod volumes;

pub use config::*;
