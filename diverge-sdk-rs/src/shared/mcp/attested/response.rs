//! An answer's `_meta`, as an agent container reads it.

use rmcp::model::MetaObject;

use super::{Error, read};
use crate::daemon::key;
use crate::shared::containers::request::Image;
use crate::shared::mcp::{IMAGE, TOOL};

/// Who answered: what the daemon attested on the `_meta` of a
/// result, of each tool of a tools list, of each resource of a
/// resources list, and of each notification, as it relayed the
/// answer to an agent. The tool is always there; its image only when
/// the daemon ran the tool itself, and not for a tool it joined
/// through a connect, whose image it never saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The answering tool's image, when known: [`IMAGE`].
    pub image: Option<Image>,
    /// The answering tool: [`TOOL`] — a record tool, by its origin
    /// and index, or a dependency tool, by its template and owner,
    /// one type for both.
    pub tool: key::Tool,
}

/// The `_meta` of an answer an agent container received, read as the
/// [`Response`] the daemon attested.
///
/// For agents in agent containers alone: it is the `_meta` of the
/// four results, of each entry of the two lists, and of each
/// notification that carry this, and nothing a tool container
/// receives does. A `_meta` without [`TOOL`], or with it or [`IMAGE`]
/// not as its type, is the [`Error`] naming the key; [`IMAGE`] absent
/// is a tool the daemon did not run, and no error.
pub fn response(meta: &MetaObject) -> Result<Response, Error> {
    let image = match meta.get(IMAGE) {
        None => None,
        Some(_) => Some(read(meta, IMAGE)?),
    };
    Ok(Response {
        image,
        tool: read(meta, TOOL)?,
    })
}
