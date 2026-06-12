//! Plugin discovery and manifest types.

mod client;
mod install_error;
mod manifest;
mod whitelist;

pub use client::*;
pub use install_error::*;
pub use manifest::*;
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
mod whitelist_tests;
