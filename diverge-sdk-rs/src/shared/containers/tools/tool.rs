//! One tool container an agent's program depends on.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::request::Image;

/// One tool container an agent's program depends on: everything the
/// caller needs to run it that the image can know, and nothing the
/// caller alone can know. A tool container depends on nothing.
///
/// A [`Container`](crate::shared::containers::request::Container) is
/// an image, two limits, mounts and arguments. The first four are
/// here, member for member; the mounts are not, because a volume is
/// named by a `volume_name` in the caller's own listing and a FUSE
/// mount is something the caller serves live, and a tool that named
/// either would be dictating the caller's storage. What the tool needs
/// of the caller that it cannot name — a mount at some path, a
/// volume of some shape, anything else about running it — it says
/// in words, in [`instructions`](Self::instructions), for whoever
/// deploys it to read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Tool {
    /// What the program calls it: the caller's handle for the tool
    /// container, and the label the caller uses when it prefixes the
    /// tool's MCP tools as it merges lists. Unique in the list. The
    /// program never finds a server by it — it calls tools by whatever
    /// names the caller's merged list shows, and knows which tool
    /// serves each by the keys under `_meta`, see
    /// [`shared::mcp`](crate::shared::mcp).
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
    /// What the caller needs to know to run it, in words, for
    /// whoever deploys the tool to read: the mounts it needs and at
    /// what paths, since it cannot name a volume or a FUSE mount of
    /// the caller's, and anything else about deploying it. Absent
    /// when the tool needs nothing said. Not read by the provider,
    /// and not handed to the container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}
