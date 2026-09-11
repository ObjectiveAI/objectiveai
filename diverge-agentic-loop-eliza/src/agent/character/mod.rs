//! Character definitions — who the agent is.
//!
//! Eliza builds its prompt from these fields EVERY turn: the
//! canonical system prompt is `system` + a `bio` block + `style`,
//! and the CHARACTER provider samples bio lines, one topic, one
//! adjective and five example groups — deterministically, keyed on
//! the room — into every message handler pass. There is no way to
//! hand Eliza a finished prompt, which is why the character is a
//! PARAMETER of the call here: what the protocol's post-transform
//! rule forbids is the agent rewriting a conversation, and the
//! character rewrites nothing — it is how Eliza speaks. A caller
//! who authored a personality upstream renders it into these.
//!
//! Absent on purpose: `username` (mention detection only, moot in
//! a room that always responds); `postExamples` (rendered only in
//! feed and thread rooms, which the loop's one DM room never is);
//! `knowledge`/`documents` (files, so MOUNTS — the documents
//! feature and `plugin-documents` ingest what is mounted); `plugins`, `settings`,
//! `secrets`, `templates` (see [the agent module](super)); and `id`
//! — the identity is the HARNESS's, pinned once per lineage in its
//! row of the caller's database, so a renamed character keeps its
//! memory.

mod character;
mod example;
mod style;

pub use character::*;
pub use example::*;
pub use style::*;
