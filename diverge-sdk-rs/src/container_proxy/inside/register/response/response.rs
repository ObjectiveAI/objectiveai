//! What `POST /register` answers with: the tools the program wants
//! beside it, each one a tool container the caller requisitions.

use crate::shared::containers::tools::Tool;
use serde::{Deserialize, Serialize};

/// The body of a `2xx` to `POST /register`.
///
/// Fixed for the container's life, as the arguments are: registration
/// happens once, and this is what the program says it needs, once.
/// An empty list — `{}` is one — is a program that needs nothing, and
/// is the usual answer of a tool container. The proxy carries the
/// list to the provider on its `Begun`, and the provider asks the
/// caller to deploy each; see [`Tool`] for what one is, and for what
/// the image can say of a tool and what it cannot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Response {
    /// The tools, in the order the program names them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}
