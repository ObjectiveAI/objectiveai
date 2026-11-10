//! The Anthropic API shapes inside Claude Code's records.
//!
//! An [`assistant`](super::assistant::Assistant) record wraps the API's own
//! message; a [`stream_event`](super::stream_event::StreamEvent) record wraps the
//! API's own streaming event; a [`user`](super::user::User) record wraps
//! the request-side message params. These are those shapes: the base
//! is `@anthropic-ai/sdk@0.39.0` — the version Claude Code's source
//! pins — widened to the CURRENT API's surface where the emitters
//! demonstrably pass newer things through verbatim: the server-tool
//! and MCP-connector content blocks, the wider citation family, the
//! typed container and context-management shapes, the iteration
//! usage, and the compaction delta. Nothing invented — every widened
//! shape is ported from a mechanical port of the newer SDK.
//!
//! - [`Message`] is `BetaMessage`, whose content is
//!   [`ContentBlock`]s and whose billing is [`Usage`].
//! - [`UserMessage`] is the `{role, content}` params object, whose
//!   content is [`ContentBlockParam`]s — the request vocabulary
//!   (images, documents, tool results).
//! - [`StreamEvent`] is `BetaRawMessageStreamEvent`, the raw
//!   streaming events.

mod assistant;
mod caller;
mod content_block;
mod document;
mod stream;
mod tool_results;
mod usage;
mod user;

pub use assistant::*;
pub use caller::*;
pub use content_block::*;
pub use document::*;
pub use stream::*;
pub use tool_results::*;
pub use usage::*;
pub use user::*;
