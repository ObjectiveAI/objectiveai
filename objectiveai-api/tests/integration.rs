//! The **single** integration-test binary for `objectiveai-api`.
//!
//! Cargo treats every `tests/*.rs` file as its own test binary, and each
//! binary that exercises the spawned `objectiveai-api` server holds its
//! own `LazyLock<ServerHandle>` (in `tests/common/server.rs`) — so
//! splitting the integration suite across multiple top-level files
//! would have spawned one api server *per* file, and running those
//! binaries concurrently saturated the OS's ephemeral-port range and
//! piled load on shared upstream resources.
//!
//! Every integration test category — `agent_completions`,
//! `vector_completions`, `functions_executions`, `functions_inventions`
//! (split internally by scalar/vector/validation/recursive),
//! `laboratories_executions`, and `asset_tests` — is a sibling-file
//! module under `tests/integration/` pulled in via `mod` declarations
//! from this entrypoint. Sibling files inside a subdirectory aren't
//! auto-discovered by cargo as separate binaries, so the whole
//! integration suite compiles into one binary and shares **exactly
//! one** `objectiveai-api` subprocess no matter how the suite is
//! invoked or how parallel cargo's test harness runs.

#![allow(clippy::too_many_arguments)]

mod common;

mod integration {
    pub mod agent_completions;
    pub mod asset_tests;
    pub mod functions_executions;
    pub mod functions_inventions_recursive;
    pub mod functions_inventions_scalar;
    pub mod functions_inventions_validation;
    pub mod functions_inventions_vector;
    pub mod laboratories_executions;
    pub mod vector_completions;
}
