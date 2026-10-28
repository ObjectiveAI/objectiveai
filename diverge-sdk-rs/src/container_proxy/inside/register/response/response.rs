//! What `POST /register` answers with: the dependencies the program
//! wants beside it, each one a tool container the caller deploys, the
//! secrets it takes, each by name to its mode, and — a tool's — the
//! permissions it holds over the daemon.

use serde::{Deserialize, Serialize};

use crate::shared::containers::dependencies::Template;
use crate::shared::containers::secrets::Declaration;
use crate::shared::permission::Grant;

/// The body of a `2xx` to `POST /register`.
///
/// Fixed for the container's life, as the arguments are: registration
/// happens once, and this is what the program says it needs, once.
/// An empty answer — `{}` is one — is a program that needs nothing,
/// takes no secret and asks the daemon for nothing. The proxy carries
/// the members to the provider on its `Begun`, and the provider asks
/// the caller to deploy each dependency, to resolve each secret and to
/// hold the program to its permissions; see [`Template`] for what a
/// dependency is, and for what the image can say of one and what it
/// cannot, [`Declaration`] for what the program says of its secrets,
/// and [`Grant`] for a permission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Response {
    /// The dependencies, in the order the program names them. A tool
    /// container's are ignored.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Template>,
    /// The secrets the program takes, each by the name its template
    /// declares it under, to the mode it takes it in: `fixed`, read at
    /// every activation, or `rotating`, held through the active
    /// period and collected at the inactive. A name the template does
    /// not declare is the registration's error. Absent, none.
    #[serde(default, skip_serializing_if = "Declaration::is_empty")]
    pub secrets: Declaration,
    /// What the program may ask of the daemon, as grants: the whole of
    /// its authority for this run, held to nothing else, and declared
    /// again at every start. A tool container's; an agent container's
    /// are ignored, since an agent asks the daemon nothing. Absent,
    /// none: everything the program asks is refused.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<Grant>,
}
