//! What `POST /register` answers with: the dependencies the program
//! wants beside it, each one a tool container the caller deploys, and
//! the secrets it takes, each by name to its mode.

use serde::{Deserialize, Serialize};

use crate::shared::containers::dependencies::Template;
use crate::shared::containers::secrets::Declaration;

/// The body of a `2xx` to `POST /register`.
///
/// Fixed for the container's life, as the arguments are: registration
/// happens once, and this is what the program says it needs, once.
/// An empty answer — `{}` is one — is a program that needs nothing
/// and takes no secret. The proxy carries both members to the
/// provider on its `Begun`, and the provider asks the caller to deploy
/// each dependency and to resolve each secret; see [`Template`] for
/// what a dependency is, and for what the image can say of one and
/// what it cannot, and [`Declaration`] for what the program says of
/// its secrets.
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
}
