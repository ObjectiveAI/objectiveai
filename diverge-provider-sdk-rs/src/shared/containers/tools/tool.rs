//! One tool container a program depends on.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::request::Image;

/// One tool container the program depends on: everything the caller
/// needs to run it that the image can know, and nothing the caller
/// alone can know.
///
/// A [`Container`](crate::shared::containers::request::Container) is
/// an image, two limits, mounts and arguments. The first four are
/// here, member for member; the mounts are not, because a volume is
/// named by a `host_name` in the caller's own listing and a FUSE
/// mount is something the caller serves live, and a tool that named
/// either would be dictating the caller's storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// What the program calls it: the MCP server it expects to find
    /// among the caller's servers, by this name. Unique in the list.
    pub name: String,
    /// The image: a name and a digest, the pair a
    /// `containers::tools::run` request names.
    pub image: Image,
    /// The memory the tool container needs, in bytes.
    pub memory: u64,
    /// The bytes the tool container may write, in bytes.
    pub disk: u64,
    /// The arguments the tool container is registered with — the
    /// `arguments` of the run request that makes it, verbatim.
    pub arguments: Value,
}
