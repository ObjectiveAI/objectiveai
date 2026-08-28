//! The Anthropic API shapes inside Claude Code's records.
//!
//! An [`assistant`](super::assistant::Assistant) record wraps the API's own
//! message; a [`stream_event`](super::stream_event::StreamEvent) record wraps the
//! API's own streaming event; a [`user`](super::user::User) record wraps
//! the request-side message params. These are those shapes, ported
//! from `@anthropic-ai/sdk@0.39.0` — the version Claude Code's source
//! pins — with nothing invented: every union here is that version's
//! union, whole.
//!
//! - [`Message`] is `BetaMessage`, whose content is
//!   [`ContentBlock`]s and whose billing is [`Usage`].
//! - [`UserMessage`] is the `{role, content}` params object, whose
//!   content is [`ContentBlockParam`]s — the request vocabulary,
//!   wider than the response one (images, documents, tool results).
//! - [`StreamEvent`] is `BetaRawMessageStreamEvent`, the six raw
//!   streaming events.

mod assistant;
mod content_block;
mod stream;
mod usage;
mod user;

pub use assistant::*;
pub use content_block::*;
pub use stream::*;
pub use usage::*;
pub use user::*;
