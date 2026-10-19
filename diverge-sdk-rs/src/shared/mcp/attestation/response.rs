//! An answer's `_meta`, as an agent container reads it.

use rmcp::model::MetaObject;

use super::{Error, Tool, read};

/// The `_meta` of an answer an agent container received, read as the
/// [`Tool`] the daemon attested: who answered, with its image when
/// the kind has one.
///
/// For agents in agent containers alone: it is the `_meta` of the
/// four results, of each tool of a tools list, of each resource of a
/// resources list, and of each notification that carry this, and
/// nothing a tool container receives does. A `_meta` without
/// [`KEY`](crate::shared::mcp::KEY) is [`Error::Missing`]; one whose value is not a tool — the agent's
/// kind, or not the type — is [`Error::Invalid`]. A connected tool is
/// no error: it is the one kind without an image, and the type says
/// so.
pub fn response(meta: &MetaObject) -> Result<Tool, Error> {
    read(meta)
}
