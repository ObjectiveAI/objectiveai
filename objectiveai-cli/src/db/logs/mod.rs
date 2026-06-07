//! Postgres-backed log writer.
//!
//! The `logs.*` schema (defined in `schema.sql`) is a hybrid:
//!
//! - **Six tier blob tables** (request + response × agent / vector /
//!   function) store the full chunk body as JSONB. Requests are
//!   written once on first chunk arrival; responses get UPSERTed per
//!   tick (the writer's shadow map skips no-op rewrites).
//! - **Fourteen streaming-content tables** carry the per-message,
//!   per-part incrementally-updating content yielded by the SDK's
//!   chunk-type `log_rows()` iterators. Diff-detection is trivial:
//!   the PKs are deterministic `(response_id, index[, sub_index])`
//!   tuples, the writer keeps a shadow map keyed the same way, and
//!   identical column values skip the UPSERT.
//!
//! The SDK exposes a zero-collection iterator over `(LogTable,
//! LogValue)` pairs for each chunk type — `AgentCompletionChunk`,
//! `VectorCompletionChunk`, and `FunctionExecutionChunk`. The writer
//! consumes one row at a time and dispatches every yielded row
//! through the same shared UPSERT path; because every agent
//! completion in the recursive tree has a globally-unique response id,
//! no parent context is needed at the writer level.
//!
//! Status: scaffolding. `LogWriter` + factory entry points exist so
//! the surrounding cli compiles. The iterator + UPSERT bodies land
//! over the next commits.

mod row;
mod writer;

pub use row::*;
pub use writer::*;
