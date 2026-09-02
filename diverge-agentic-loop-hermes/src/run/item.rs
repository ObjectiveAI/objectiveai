//! What the runner yields.

use diverge_provider_sdk::endpoints::agentic_loop::run::server::response::AgenticLoopChunk;

use crate::filesystem::Export;

/// One item of the run's stream: a chunk of the conversation; a
/// resource the run rewrote, whole, under its field's name; or a
/// piece of the continuation — the closer, last.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// One chunk of the loop's output.
    Chunk(AgenticLoopChunk),
    /// A resource as the run left it. The driver sends it as
    /// `Response::Resource`.
    Resource {
        /// The request field's dotted path.
        name: &'static str,
        /// The resource's new content, whole.
        body: Vec<u8>,
    },
    /// One piece of the continuation, tagged. The driver sends each
    /// as `Response::Continuation`; nothing follows the last.
    Continuation(Vec<u8>),
}

impl From<Export> for Item {
    fn from(export: Export) -> Self {
        match export {
            Export::Resource { name, body } => Item::Resource { name, body },
            Export::Continuation(piece) => Item::Continuation(piece),
        }
    }
}
