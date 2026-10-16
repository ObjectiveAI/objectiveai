//! One dependency, as an image declares it.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Mode;
use super::super::request::Image;
use crate::shared::permission::Grant;

/// One tool container an agent's program depends on: everything the
/// caller needs to deploy it, as fields and not as words. A tool
/// container depends on nothing.
///
/// A [`Container`](crate::shared::containers::request::Container) is an
/// image, two limits, mounts, arguments and secrets. The image, the
/// limits and the arguments are here, member for member; the secrets by
/// name alone, their values the caller's — each the value of the vault
/// key the agent that spawns the tool binds the same name to, read at
/// every start. The mounts are not the caller's volumes — a volume is
/// named in the caller's own listing, and a tool that named one would
/// be dictating the caller's storage — but, for a transient tool alone,
/// the AGENT's own files and directories, each a path in the agent
/// container served live at a path in the tool container: the caller
/// serves the agent container, a `containers::fuse_serve` of the path,
/// and mounts the tool, a `fuse::mount` answered from it.
/// [`mode`](Self::mode) says which of the two modes the dependency is —
/// a transient tool of the agent's run, with those mounts, or a service
/// the caller keeps, with a scope — see [`Mode`]; and
/// [`permissions`](Self::permissions) what its account may do — grants
/// that name nothing, so the template is the same on every daemon it
/// reaches, and among them the word `self`, which reaches the agent a
/// transient tool is the dependency of and every agent attached to a
/// service.
///
/// There are no instructions. What a template cannot say — the provider
/// the tool runs on, the volumes it is given, the account's name — is
/// the caller's, and the caller needs no prose to choose it. The
/// daemon's own tool templates are another thing: a record the daemon
/// keeps, named by a hash, made by a request; this is what an image
/// declares and a caller deploys.
///
/// There is no name. A dependency is named by its ID: the lowercase
/// hexadecimal SHA-256 of its [`canonical`](crate::shared::canonical)
/// bytes — this template as compact JSON, absent members omitted, the
/// image's `references` absent too, every object key sorted at every
/// depth, the `arguments` and the `secrets` included, the `mode` and
/// its members among the keys — sixty-four characters, which the caller
/// computes over [`hashed`](Self::hashed). Two declarations with one id
/// are one dependency, and a list that declares one twice is refused;
/// the same template deployed for the same agent, on whatever provider,
/// is the same dependency, and the same service template declared under
/// the same scope key, by whatever agent, is the same service. The
/// caller labels the tool's MCP tools by the id when it merges lists,
/// and the program knows which tool serves each by the keys under
/// `_meta`, see [`shared::mcp`](crate::shared::mcp).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
pub struct Template {
    /// The image: a digest and its references, as a
    /// `containers::tools::run` request names one.
    pub image: Image,
    /// The memory the tool container needs, in bytes.
    pub memory: u64,
    /// The bytes the tool container may write, in bytes.
    pub disk: u64,
    /// The arguments the tool container is registered with — the
    /// `arguments` of the run request that makes it, verbatim.
    pub arguments: Value,
    /// The secrets the image needs, by name, in the order the image
    /// states them: what every container of this dependency is handed
    /// beside the arguments, each name with the value of the vault
    /// key the agent the tool is spawned for binds the same name to.
    /// Inside the hash. A name is not empty, no two are alike, and
    /// every one is a name the registering agent's template declares;
    /// a template that says otherwise is the registration's error.
    /// Absent, the image needs none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secrets: Vec<String>,
    /// Which mode of dependency this is, with that mode's own members
    /// flat beside the `mode` that names it: a transient tool's mounts,
    /// or a service's scope. See [`Mode`].
    #[serde(flatten)]
    pub mode: Mode,
    /// What the tool's account may do, as grants that name nothing: see
    /// [`Grant`]. The caller gives the tool an account holding exactly
    /// these. Absent when empty, and then the tool may do nothing
    /// through the daemon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Grant>,
}

impl Template {
    /// The template as its id hashes it: the same, with the image's
    /// references absent, since where the bytes may be fetched is not
    /// what the tool is.
    pub fn hashed(&self) -> Template {
        Template {
            image: self.image.hashed(),
            ..self.clone()
        }
    }
}
