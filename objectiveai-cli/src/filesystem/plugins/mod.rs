//! Plugin discovery and manifest types.

mod client;
#[cfg(feature = "http")]
mod install_error;
mod manifest;
mod platform;
mod whitelist;

pub use client::*;
#[cfg(feature = "http")]
pub use install_error::*;
pub use manifest::*;
pub use platform::*;
pub use whitelist::*;

#[cfg(test)]
mod client_tests;
#[cfg(all(test, feature = "http"))]
mod install_error_tests;
#[cfg(all(test, feature = "http"))]
mod install_tests;
#[cfg(test)]
mod manifest_tests;
#[cfg(test)]
mod platform_tests;
#[cfg(test)]
mod whitelist_tests;
