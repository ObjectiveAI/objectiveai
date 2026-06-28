//! Podman integration.
//!
//! [`install`] obtains the podman executable (downloaded + installed on
//! first use, gated for concurrency); driven by
//! [`crate::context::Context::podman`]. Further podman-related code —
//! machine/VM setup, running containers, etc. — will live in sibling
//! modules here.

pub mod install;
