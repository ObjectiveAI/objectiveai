//! Profile types for Function execution.
//!
//! A Profile contains the learned weights and configuration needed to execute
//! a Function. Profiles are typically trained on example data to optimize
//! scoring behavior.

use crate::vector;
use serde::{Deserialize, Serialize};

/// A Profile definition, either remote (GitHub-hosted) or inline.
///
/// Profiles contain the weights and nested configurations needed to execute
/// a Function. They correspond to a Function's task structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Profile {
    /// A GitHub-hosted profile with metadata.
    Remote(RemoteProfile),
    /// An inline profile definition.
    Inline(InlineProfile),
}

/// A GitHub-hosted profile with full metadata.
///
/// Stored as `profile.json` in GitHub repositories and referenced by
/// `owner/repository`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteProfile {
    /// Human-readable description of the profile.
    pub description: String,
    /// Version history and changes for this profile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<String>,
    /// Configuration for each task in the corresponding Function.
    pub tasks: Vec<TaskProfile>,
}

/// An inline profile definition without metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineProfile {
    /// Configuration for each task in the corresponding Function.
    pub tasks: Vec<TaskProfile>,
}

/// Configuration for a single task within a Profile.
///
/// Each variant corresponds to a task type in the Function definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TaskProfile {
    /// Profile for a nested function task (references another profile).
    RemoteFunction {
        /// GitHub repository owner.
        owner: String,
        /// GitHub repository name.
        repository: String,
        /// Git commit SHA. Highly recommended for GitHub-hosted profiles to
        /// ensure compatibility if the referenced profile's shape changes.
        commit: Option<String>,
    },
    /// Inline profile for a nested function task.
    InlineFunction(InlineProfile),
    /// Configuration for a vector completion task.
    VectorCompletion {
        /// The ensemble to use for voting.
        ensemble: vector::completions::request::Ensemble,
        /// Weights for each LLM in the ensemble.
        profile: Vec<rust_decimal::Decimal>,
    },
}

impl TaskProfile {
    /// Returns `false` if any remote function reference is missing a commit SHA.
    ///
    /// While not strictly required, specifying commit SHAs is highly recommended
    /// for GitHub-hosted profiles to ensure the profile remains valid if the
    /// referenced profile's structure changes.
    pub fn validate_commit_required(&self) -> bool {
        match self {
            TaskProfile::RemoteFunction { commit, .. } => commit.is_some(),
            TaskProfile::InlineFunction(inline) => inline
                .tasks
                .iter()
                .all(TaskProfile::validate_commit_required),
            TaskProfile::VectorCompletion { .. } => true,
        }
    }
}
