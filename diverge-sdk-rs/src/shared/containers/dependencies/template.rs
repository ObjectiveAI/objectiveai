//! One tool container an agent's program depends on, as a template
//! the caller deploys.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Database, Mount};
use super::super::request::Image;
use crate::shared::permission::Grant;

/// One tool container an agent's program depends on: everything the
/// caller needs to deploy it, as fields and not as words. A tool
/// container depends on nothing.
///
/// A [`Container`](crate::shared::containers::request::Container) is
/// an image, two limits, mounts and arguments. The image, the limits
/// and the arguments are here, member for member. The mounts are not
/// the caller's volumes — a volume is named in the caller's own
/// listing, and a tool that named one would be dictating the caller's
/// storage — but the AGENT's own files and directories,
/// [`fuse_file_mounts`](Self::fuse_file_mounts) and
/// [`fuse_directory_mounts`](Self::fuse_directory_mounts), each a path
/// in the agent container served live at a path in the tool container:
/// the caller serves the agent container, a `containers::serve` of the
/// path, and mounts the tool, a `fuse::mount` answered from it. Beside
/// them, [`database`](Self::database) says which database scope the
/// tool gets, and [`permissions`](Self::permissions) what its account
/// may do — grants that name nothing, so the template is the same on
/// every daemon it reaches.
///
/// There are no instructions. What a template cannot say — the
/// provider the tool runs on, the volumes it is given, the account's
/// name — is the caller's, and the caller needs no prose to choose it.
/// The daemon's own tool templates are another thing: a record the
/// daemon keeps, named by a hash, made by a request; this is what an
/// image declares and a caller deploys.
///
/// There is no name. A dependency is named by its ID: the lowercase
/// hexadecimal SHA-256 of its [`canonical`](crate::shared::canonical)
/// bytes — this template as compact JSON, absent members omitted,
/// every object key sorted at every depth, the `arguments` included
/// — sixty-four characters, which the caller computes. Two
/// declarations with one id are one dependency, and a list that
/// declares one twice is refused; the same template deployed for the
/// same agent, on whatever provider, is the same dependency with the
/// same database scope. The caller labels the tool's MCP tools by the
/// id when it merges lists, and the program knows which tool serves
/// each by the keys under `_meta`, see [`shared::mcp`](crate::shared::mcp).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Template {
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
    /// Which database scope the tool gets: one per parent agent, or
    /// one shared by every parent made from the same agent template.
    /// See [`Database`].
    pub database: Database,
    /// Files of the agent container served live into the tool
    /// container, one each over FUSE: see [`Mount`]. The agent's path
    /// names a file; the tool's path is where it appears. Absent when
    /// empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_file_mounts: Vec<Mount>,
    /// Directories of the agent container served live into the tool
    /// container, one each over FUSE: see [`Mount`]. The agent's path
    /// names a directory, whose whole tree the tool sees; the tool's
    /// path is where it appears. Absent when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fuse_directory_mounts: Vec<Mount>,
    /// What the tool's account may do, as grants that name nothing:
    /// see [`Grant`]. The caller gives the tool an account holding
    /// exactly these. Absent when empty, and then the tool may do
    /// nothing through the daemon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Grant>,
}
