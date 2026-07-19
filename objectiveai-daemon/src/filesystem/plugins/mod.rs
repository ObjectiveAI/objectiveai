//! Plugin discovery and manifest types.

mod client;
mod manifest;

pub use manifest::*;

#[cfg(test)]
mod client_tests;
#[cfg(test)]
mod install_tests;
#[cfg(test)]
mod manifest_tests;
