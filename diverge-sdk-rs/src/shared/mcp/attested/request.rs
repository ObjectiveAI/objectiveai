//! A request's `_meta`, as a tool container reads it.

use rmcp::model::MetaObject;
use serde::Deserialize;

use super::Error;
use crate::daemon::key;
use crate::shared::containers::request::Image;
use crate::shared::mcp::{AGENT, IMAGE};

/// Who is calling: what the daemon attested on the `_meta` of a
/// `list_tools`, `list_resources`, `call_tool` or `read_resource`
/// request before forwarding it to a tool. An agent is always run
/// from its image, so both are always there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The calling agent's image: [`IMAGE`].
    pub image: Image,
    /// The calling agent: [`AGENT`].
    pub agent: key::Agent,
}

/// The `_meta` of a request a tool container received, read as the
/// [`Request`] the daemon attested.
///
/// For tools in tool containers alone: it is the params' `_meta` of
/// the four requests that carry this, and nothing an agent container
/// receives does. A `_meta` without [`IMAGE`] or [`AGENT`], or with
/// either not as its type, is the [`Error`] naming the key.
pub fn request(meta: &MetaObject) -> Result<Request, Error> {
    Ok(Request {
        image: read(meta, IMAGE)?,
        agent: read(meta, AGENT)?,
    })
}

/// The value under `key`, as `T`.
pub(super) fn read<'a, T: Deserialize<'a>>(
    meta: &'a MetaObject,
    key: &'static str,
) -> Result<T, Error> {
    let value = meta.get(key).ok_or(Error::Missing(key))?;
    T::deserialize(value).map_err(|source| Error::Invalid { key, source })
}
