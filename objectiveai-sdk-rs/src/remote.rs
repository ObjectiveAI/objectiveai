//! Remote source types for function, profile, and agent hosting.

use serde::{Deserialize, Serialize};
use std::fmt;
use schemars::JsonSchema;

/// The remote source where a function, profile, or agent is hosted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, arbitrary::Arbitrary)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "Remote")]
pub enum Remote {
    /// GitHub repository.
    #[schemars(title = "Github")]
    Github,
    /// Local filesystem.
    #[schemars(title = "Filesystem")]
    Filesystem,
    /// Mock (for testing).
    #[schemars(title = "Mock")]
    Mock,
}

impl fmt::Display for Remote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Remote::Github => write!(f, "github"),
            Remote::Filesystem => write!(f, "filesystem"),
            Remote::Mock => write!(f, "mock"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "remote", rename_all = "snake_case")]
#[schemars(rename = "RemotePath")]
pub enum RemotePath {
    #[schemars(title = "Github")]
    Github {
        owner: String,
        repository: String,
        commit: String,
    },
    #[schemars(title = "Filesystem")]
    Filesystem {
        owner: String,
        repository: String,
        commit: String,
    },
    #[schemars(title = "Mock")]
    Mock {
        name: String,
    },
}

impl RemotePath {
    pub fn remote(&self) -> Remote {
        match self {
            RemotePath::Github { .. } => Remote::Github,
            RemotePath::Filesystem { .. } => Remote::Filesystem,
            RemotePath::Mock { .. } => Remote::Mock,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            RemotePath::Github { repository, .. } => repository,
            RemotePath::Filesystem { repository, .. } => repository,
            RemotePath::Mock { name } => name,
        }
    }

    pub fn key(&self) -> String {
        match self {
            RemotePath::Github { owner, repository, commit } => {
                format!("{}/{}/{}/{}", self.remote(), owner, repository, commit)
            }
            RemotePath::Filesystem { owner, repository, commit } => {
                format!("{}/{}/{}/{}", self.remote(), owner, repository, commit)
            }
            RemotePath::Mock { name } => {
                format!("{}/{}", self.remote(), name)
            }
        }
    }

    pub fn url(&self) -> String {
        match self {
            RemotePath::Github { owner, repository, commit } => format!(
                "[{}](https://github.com/{}/{}/commit/{})",
                repository, owner, repository, commit
            ),
            RemotePath::Filesystem { owner, repository, commit } => format!(
                "[{}](file://{}/{}) ({})",
                repository, owner, repository, commit
            ),
            RemotePath::Mock { name } => format!(
                "[{}](mock://{})",
                name, name
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, JsonSchema, arbitrary::Arbitrary)]
#[serde(tag = "remote", rename_all = "snake_case")]
#[schemars(rename = "RemotePathCommitOptional")]
pub enum RemotePathCommitOptional {
    #[schemars(title = "Github")]
    Github {
        owner: String,
        repository: String,
        commit: Option<String>,
    },
    #[schemars(title = "Filesystem")]
    Filesystem {
        owner: String,
        repository: String,
        commit: Option<String>,
    },
    #[schemars(title = "Mock")]
    Mock {
        name: String,
    },
}

impl RemotePathCommitOptional {
    pub fn remote(&self) -> Remote {
        match self {
            RemotePathCommitOptional::Github { .. } => Remote::Github,
            RemotePathCommitOptional::Filesystem { .. } => Remote::Filesystem,
            RemotePathCommitOptional::Mock { .. } => Remote::Mock,
        }
    }
}

impl From<RemotePath> for RemotePathCommitOptional {
    fn from(path: RemotePath) -> Self {
        match path {
            RemotePath::Github { owner, repository, commit } => {
                RemotePathCommitOptional::Github { owner, repository, commit: Some(commit) }
            }
            RemotePath::Filesystem { owner, repository, commit } => {
                RemotePathCommitOptional::Filesystem { owner, repository, commit: Some(commit) }
            }
            RemotePath::Mock { name } => {
                RemotePathCommitOptional::Mock { name }
            }
        }
    }
}
