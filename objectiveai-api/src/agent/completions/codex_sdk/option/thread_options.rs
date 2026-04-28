use serde::{Deserialize, Serialize};

/// Per-thread options. Mirrors `ThreadOptions` in the Python SDK. Accepts
/// both snake_case and camelCase keys on deserialize.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThreadOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(
        rename = "sandbox_mode",
        alias = "sandboxMode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sandbox_mode: Option<super::SandboxMode>,

    #[serde(
        rename = "working_directory",
        alias = "workingDirectory",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub working_directory: Option<String>,

    #[serde(
        rename = "skip_git_repo_check",
        alias = "skipGitRepoCheck",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub skip_git_repo_check: Option<bool>,

    #[serde(
        rename = "model_reasoning_effort",
        alias = "modelReasoningEffort",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub model_reasoning_effort: Option<super::ModelReasoningEffort>,

    #[serde(
        rename = "network_access_enabled",
        alias = "networkAccessEnabled",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub network_access_enabled: Option<bool>,

    #[serde(
        rename = "web_search_enabled",
        alias = "webSearchEnabled",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub web_search_enabled: Option<bool>,

    #[serde(
        rename = "approval_policy",
        alias = "approvalPolicy",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub approval_policy: Option<super::ApprovalMode>,

    #[serde(
        rename = "additional_directories",
        alias = "additionalDirectories",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_directories: Option<Vec<String>>,
}
