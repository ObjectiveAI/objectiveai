//! Profile types for Function execution.
//!
//! A Profile contains the learned weights and configuration needed to execute
//! a Function. Profiles are typically trained on example data to optimize
//! scoring behavior.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A profile specification that is either an inline profile definition
/// or a remote path reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.InlineProfileOrRemoteCommitOptional")]
pub enum InlineProfileOrRemoteCommitOptional {
    #[schemars(title = "Inline")]
    Inline(InlineProfile),
    #[schemars(title = "Remote")]
    Remote(crate::RemotePathCommitOptional),
    /// A bare favorite name. Resolves at handler time.
    #[schemars(title = "Favorite")]
    Favorite(String),
}

/// A Profile definition, either remote or inline.
///
/// Profiles contain the weights and nested configurations needed to execute
/// a Function. They correspond to a Function's task structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.Profile")]
pub enum Profile {
    /// A remote profile with metadata.
    #[schemars(title = "Remote")]
    Remote(RemoteProfile),
    /// An inline profile definition.
    #[schemars(title = "Inline")]
    Inline(InlineProfile),
}

/// A remote profile, either tasks-based or auto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.RemoteProfile")]
pub enum RemoteProfile {
    /// Tasks-based profile with per-task configuration.
    #[schemars(title = "Tasks")]
    Tasks(RemoteTasksProfile),
    /// Auto profile that applies a single swarm+weights to all vector completion tasks.
    #[schemars(title = "Auto")]
    Auto(crate::swarm::RemoteSwarmBase),
}

/// An inline profile, either tasks-based or auto.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.InlineProfile")]
pub enum InlineProfile {
    /// Tasks-based profile with per-task configuration.
    #[schemars(title = "Tasks")]
    Tasks(InlineTasksProfile),
    /// Auto profile that applies a single swarm+weights to all vector completion tasks.
    #[schemars(title = "Auto")]
    Auto(crate::swarm::InlineSwarmBase),
}

impl<'a> arbitrary::Arbitrary<'a> for InlineProfile {
    fn arbitrary(
        u: &mut arbitrary::Unstructured<'a>,
    ) -> arbitrary::Result<Self> {
        // Only generate Tasks variant since Auto requires SwarmBaseWithProfile
        // which has complex dependencies.
        Ok(InlineProfile::Tasks(u.arbitrary()?))
    }
}

/// An inline tasks-based profile definition without metadata.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Serialize,
    Deserialize,
    JsonSchema,
    arbitrary::Arbitrary,
)]
#[schemars(rename = "functions.InlineTasksProfile")]
pub struct InlineTasksProfile {
    /// Configuration for each task in the corresponding Function.
    pub tasks: Vec<TaskProfile>,
    /// Optional weights for each Task in the corresponding Function.
    /// If `None`, uniform weights are used.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(extend("omitempty" = true))]
    pub weights: Option<crate::Weights>,
}

/// A remote tasks-based profile with full metadata.
///
/// Stored as `profile.json` in repositories and referenced by
/// `remote/owner/repository`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[schemars(rename = "functions.RemoteTasksProfile")]
pub struct RemoteTasksProfile {
    /// Human-readable description of the profile.
    pub description: String,
    #[serde(flatten)]
    #[schemars(schema_with = "crate::flatten_schema::<InlineTasksProfile>")]
    pub inner: InlineTasksProfile,
}

/// Configuration for a single task within a Profile.
///
/// Each variant corresponds to a task type in the Function definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "functions.TaskProfile")]
pub enum TaskProfile {
    /// Profile for a nested function task (references another profile).
    #[schemars(title = "Remote")]
    Remote(crate::RemotePath),
    /// Inline profile for a task (tasks-based or auto).
    #[schemars(title = "Inline")]
    Inline(InlineProfile),
    /// Placeholder task — no configuration needed, output is fixed.
    #[schemars(title = "Placeholder")]
    Placeholder {},
}

impl<'a> arbitrary::Arbitrary<'a> for TaskProfile {
    fn arbitrary(
        u: &mut arbitrary::Unstructured<'a>,
    ) -> arbitrary::Result<Self> {
        // Inline variant is recursive (InlineProfile → InlineTasksProfile → Vec<TaskProfile>),
        // so use go-deep bool to make recursion exponentially rare.
        if u.arbitrary().unwrap_or(false) {
            Ok(TaskProfile::Inline(u.arbitrary()?))
        } else if u.arbitrary().unwrap_or(false) {
            Ok(TaskProfile::Remote(u.arbitrary()?))
        } else {
            Ok(TaskProfile::Placeholder {})
        }
    }
}
