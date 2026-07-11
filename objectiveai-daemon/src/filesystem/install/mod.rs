//! Shared GitHub-install engine for tools and plugins. Holds the
//! generic install engine + [`InstallManifest`] trait, the install
//! error type, the install/identifier validators, and the install
//! whitelist. The per-kind discovery + wrappers live in
//! `filesystem::tools` and `filesystem::plugins`.

mod engine;
mod error;
mod identifiers;
mod whitelist;

pub use engine::*;
pub use error::*;
pub use identifiers::*;
pub use whitelist::*;

#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod whitelist_tests;
