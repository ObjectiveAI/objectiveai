//! Postgres-backed log writer.
//!
//! The `logs.*` schema (defined in `schema.sql`) is a hybrid:
//!
//! - **Six tier blob tables** (request + response × agent / vector /
//!   function) store the full chunk body as JSONB. Requests are
//!   written once on first chunk arrival; responses get UPDATEd per
//!   tick when their body changes.
//! - **Fourteen streaming-content tables** carry the per-message,
//!   per-part incrementally-updating content yielded by the SDK's
//!   chunk-type `log_rows()` iterators.
//!
//! Diff detection: every row written has a deterministic PK shape
//! `(response_id, index[, sub_index])`. The writer holds a shadow map
//! keyed identically and hashes each row's body columns. Three
//! verdicts come out: `Insert` (first sight), `Update` (changed
//! body), `Skip` (unchanged). The SQL helpers in [`write`] dispatch
//! flat INSERT or UPDATE per verdict — no `ON CONFLICT` ambiguity.
//!
//! The writer is the sole caller and is single-instance per stream,
//! so the shadow's verdict is authoritative — concurrent races on
//! the same row id can't happen.

mod lookup;
mod row;
mod rows;
mod shadow;
mod write;
mod writer;

pub use lookup::*;
pub use row::*;
pub use rows::*;
pub use shadow::*;
pub use write::*;
pub use writer::*;
