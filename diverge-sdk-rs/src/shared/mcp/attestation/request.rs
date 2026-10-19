//! A request's `_meta`, as a tool container reads it.

use rmcp::model::MetaObject;
use serde::Deserialize;

use super::{Caller, Error};
use crate::shared::mcp::KEY;

/// The `_meta` of a request a tool container received, read as the
/// [`Caller`] the daemon attested: the agent calling from inside the
/// harness, with its image, or the caller from outside, by the
/// identity the daemon authorized it as.
///
/// For tools in tool containers alone: it is the params' `_meta` of
/// the four requests — `list_tools`, `list_resources`, `call_tool`,
/// `read_resource` — that carry this, and nothing an agent container
/// receives does. A `_meta` without [`KEY`] is [`Error::Missing`];
/// one whose value is not a caller — a tool's kind, or not the type —
/// is [`Error::Invalid`].
pub fn request(meta: &MetaObject) -> Result<Caller, Error> {
    read(meta)
}

/// The value under [`KEY`], as `T`.
pub(super) fn read<'a, T: Deserialize<'a>>(meta: &'a MetaObject) -> Result<T, Error> {
    let value = meta.get(KEY).ok_or(Error::Missing)?;
    T::deserialize(value).map_err(Error::Invalid)
}
