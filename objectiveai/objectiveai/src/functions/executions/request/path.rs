//! Path parameters for function execution requests.
//!
//! These specify the GitHub repository references for remote Functions
//! and Profiles.

use serde::{Deserialize, Serialize};

/// Path parameters for remote Function with inline Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileInlineRequestPath {
    /// Function repository owner.
    pub fowner: String,
    /// Function repository name.
    pub frepository: String,
    /// Function Git commit SHA (optional).
    pub fcommit: Option<String>,
}

/// Path parameters for inline Function with remote Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInlineProfileRemoteRequestPath {
    /// Profile repository owner.
    pub powner: String,
    /// Profile repository name.
    pub prepository: String,
    /// Profile Git commit SHA (optional).
    pub pcommit: Option<String>,
}

/// Path parameters for remote Function with remote Profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRemoteProfileRemoteRequestPath {
    /// Function repository owner.
    pub fowner: String,
    /// Function repository name.
    pub frepository: String,
    /// Function Git commit SHA (optional).
    pub fcommit: Option<String>,
    /// Profile repository owner.
    pub powner: String,
    /// Profile repository name.
    pub prepository: String,
    /// Profile Git commit SHA (optional).
    pub pcommit: Option<String>,
}
