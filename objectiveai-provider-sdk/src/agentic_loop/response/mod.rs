//! Agentic loop response data.
//!
//! A response is a **stream** of [`AgenticLoopChunk`]s. Each chunk is
//! one event — content, reasoning, a tool call, a refusal, a tool
//! result, usage, an error, or the continuation — rather than a
//! partially-filled record of everything that could have happened.
//!
//! [`response_old`](super::response_old) holds the mirror of the
//! agent-completions types, kept alongside as the reference this
//! replaces rather than as something to migrate wholesale.

mod chunk;
mod continuation_chunk;

pub use chunk::*;
pub use continuation_chunk::*;
