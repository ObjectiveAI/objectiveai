//! Postgres-backed log writer.
//!
//! Six request/response tables plus six content-addressed leaf tables
//! (`text`, `image`, `audio`, `video`, `file`, `input`) under the
//! `logs.*` schema receive every chunk the cli produces. The writer
//! strips embedded sub-completions to `LogRef { table, id }` and uses
//! an in-memory shadow map keyed by `chunk.id` to skip re-inserting
//! rows whose stripped bodies haven't changed since the last chunk.
//!
//! Status: scaffolding. The factory entry points + writer API exist
//! so the surrounding cli compiles; the strip+insert bodies land in
//! follow-up commits.

mod row;
mod writer;

pub use row::*;
pub use writer::*;
