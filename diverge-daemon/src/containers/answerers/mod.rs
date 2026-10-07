//! The eight kinds of ask a provider makes on a run scope, answered.
//!
//! One type, [`Answerer`], is every one of the provider client's
//! answerer traits for one run: it holds what the asks need — the
//! daemon, which container, its account, the mounts, the served
//! tools, the chain and the deployer — and each trait's impl is one
//! file. Real: the authorizations, from the tool's admissions; the
//! tool deployer; the `/daemon` pair, served by the same session a
//! socket gets; MCP, the served tools merged; FUSE, the mounts. The
//! truth, which is nothing: OCI — the daemon holds no image, so the
//! provider pulls from its registries. Declined until their steps:
//! Postgres, and the vault.
//!
//! Its own files are flattened into it, so everything is named
//! through this module and not through the file it lives in.

mod answerer;
mod authorizer;
mod daemon;
mod deployer;
mod fuse;
mod mcp;
mod oci;
mod postgres;
mod vault;

pub use answerer::*;
