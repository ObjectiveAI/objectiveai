//! Local-filesystem tools — lightweight executables a user drops into
//! `<base_dir>/tools/` for later exposure to agents (e.g. as MCP
//! tools). Mirrors `filesystem::plugins` minus the install pipeline:
//! tools are hand-placed, not fetched from GitHub.

mod client;
mod manifest;

pub use client::*;
pub use manifest::*;
