//! What is done to dependency templates that exist, and to the tools
//! running from them.

use serde::{Deserialize, Serialize};

/// The actions over dependency templates that exist, which a grant
/// reaches as far as its `within` says. An action over a tool running
/// from a template is judged by the grant's reach over the template.
/// Snake case on the wire: `"get"`, `"delete"`, `"list"`,
/// `"download"`, `"upload"`, `"transfer"`, `"filetree"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Over {
    /// Get one, as
    /// [`dependencies::get`](crate::daemon::endpoints::dependencies::get)
    /// does.
    Get,
    /// Delete one, as
    /// [`dependencies::delete`](crate::daemon::endpoints::dependencies::delete)
    /// does.
    Delete,
    /// List them, as
    /// [`dependencies::list`](crate::daemon::endpoints::dependencies::list)
    /// does; the list sends what the grant reaches.
    List,
    /// Send the client files out of a tool running from one, as
    /// [`dependencies::download`](crate::daemon::endpoints::dependencies::download)
    /// does.
    Download,
    /// Put files into a tool running from one, as
    /// [`dependencies::upload`](crate::daemon::endpoints::dependencies::upload)
    /// does, and as a transfer landing in one does.
    Upload,
    /// Copy files out of a tool running from one, as
    /// [`dependencies::transfer`](crate::daemon::endpoints::dependencies::transfer)
    /// does; where they land is judged by its own grant.
    Transfer,
    /// Watch a tool running from one whole, as
    /// [`dependencies::filetree`](crate::daemon::endpoints::dependencies::filetree)
    /// does.
    Filetree,
}
