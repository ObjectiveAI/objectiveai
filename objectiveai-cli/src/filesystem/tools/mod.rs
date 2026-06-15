//! Local-filesystem tools — lightweight executables exposed to agents
//! (e.g. as MCP tools). A tool is either hand-placed under
//! `<base_dir>/tools/` or fetched from GitHub via the shared install
//! engine ([`crate::filesystem::install`]); `install` holds the tool
//! side of that pipeline (impl-only, hence `pub mod`).

mod client;
pub mod install;
mod manifest;

pub use client::*;
pub use manifest::*;

#[cfg(test)]
mod install_tests;
