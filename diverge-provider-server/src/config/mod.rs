//! The provider's configuration: where it lives, how it is found, and
//! what it says.
//!
//! - `--config <dir>`, else `DIVERGE_PROVIDER_CONFIG`, else
//!   `~/.diverge/provider/`, names the provider's DIRECTORY, created
//!   if absent.
//! - `<dir>/config.yaml` is read if present; absent means the built-in
//!   defaults. No other name or extension is looked for.
//! - Every path inside `config.yaml` resolves relative to `<dir>`,
//!   never to the working directory.
//! - `<dir>/hooks/`, `<dir>/logs/`, `<dir>/data/` and `<dir>/run/` sit
//!   beside it; `data` is the one the file may move. Every folder
//!   under `hooks/` is a hook, and the file names one by that
//!   folder's name alone — see [`hook`](crate::hook).
//!
//! [`Config`] is the document; [`auth`], [`clients`] and [`volumes`]
//! its sections.
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
pub mod volumes;

pub use config::*;
