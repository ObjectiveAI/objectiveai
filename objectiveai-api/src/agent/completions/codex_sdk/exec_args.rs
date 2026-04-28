use indexmap::IndexMap;

/// Arguments for one `codex exec` invocation. The 1:1 Rust port of
/// `CodexExecArgs` in the Python SDK (`exec.py`). Fields not represented as
/// CLI flags (`base_url`, `api_key`) are forwarded as environment variables
/// — see [`Self::to_env`].
///
/// `input` is written to the child's stdin, not passed as a flag.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodexExecArgs {
    /// The user prompt — written to the child's stdin.
    pub input: String,

    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub thread_id: Option<String>,

    pub images: Option<Vec<String>>,
    pub model: Option<String>,
    pub sandbox_mode: Option<super::SandboxMode>,
    pub working_directory: Option<String>,
    pub additional_directories: Option<Vec<String>>,
    pub skip_git_repo_check: Option<bool>,
    pub output_schema_file: Option<String>,
    pub model_reasoning_effort: Option<super::ModelReasoningEffort>,
    pub network_access_enabled: Option<bool>,
    pub web_search_enabled: Option<bool>,
    pub approval_policy: Option<super::ApprovalMode>,
}

impl CodexExecArgs {
    /// Build the argv that follows the codex binary path. Mirrors
    /// `_build_command_args` in `exec.py:51-100` exactly, including
    /// argument order (which the Python SDK's tests rely on).
    pub fn to_command_args(&self) -> Vec<String> {
        let mut args: Vec<String> = vec!["exec".into(), "--experimental-json".into()];

        if let Some(model) = &self.model {
            args.push("--model".into());
            args.push(model.clone());
        }

        if let Some(sandbox_mode) = self.sandbox_mode {
            args.push("--sandbox".into());
            args.push(sandbox_mode_value(sandbox_mode).into());
        }

        if let Some(cwd) = &self.working_directory {
            args.push("--cd".into());
            args.push(cwd.clone());
        }

        if let Some(dirs) = &self.additional_directories {
            for dir in dirs {
                args.push("--add-dir".into());
                args.push(dir.clone());
            }
        }

        if self.skip_git_repo_check == Some(true) {
            args.push("--skip-git-repo-check".into());
        }

        if let Some(schema_path) = &self.output_schema_file {
            args.push("--output-schema".into());
            args.push(schema_path.clone());
        }

        if let Some(effort) = self.model_reasoning_effort {
            args.push("--config".into());
            args.push(format!(
                "model_reasoning_effort=\"{}\"",
                reasoning_effort_value(effort)
            ));
        }

        if let Some(enabled) = self.network_access_enabled {
            args.push("--config".into());
            args.push(format!(
                "sandbox_workspace_write.network_access={}",
                bool_lit(enabled)
            ));
        }

        if let Some(enabled) = self.web_search_enabled {
            args.push("--config".into());
            args.push(format!("features.web_search_request={}", bool_lit(enabled)));
        }

        if let Some(policy) = self.approval_policy {
            args.push("--config".into());
            args.push(format!(
                "approval_policy=\"{}\"",
                approval_mode_value(policy)
            ));
        }

        if let Some(images) = &self.images {
            for image in images {
                args.push("--image".into());
                args.push(image.clone());
            }
        }

        if let Some(thread_id) = &self.thread_id {
            args.push("resume".into());
            args.push(thread_id.clone());
        }

        args
    }

    /// Build the environment for the child process. `base` is the inherited
    /// environment from the caller; the originator marker, `base_url`, and
    /// `api_key` are layered on top, mirroring `_build_env` in
    /// `exec.py:102-118`.
    pub fn to_env(&self, mut base: IndexMap<String, String>) -> IndexMap<String, String> {
        // exec.py:109-110 — set the originator marker if the caller hasn't
        // overridden it. Identifies us as the Rust SDK in upstream telemetry.
        base.entry(INTERNAL_ORIGINATOR_ENV.into())
            .or_insert_with(|| RUST_SDK_ORIGINATOR.into());

        if let Some(url) = &self.base_url {
            base.insert("OPENAI_BASE_URL".into(), url.clone());
        }
        if let Some(key) = &self.api_key {
            base.insert("CODEX_API_KEY".into(), key.clone());
        }
        base
    }
}

/// Mirrors `INTERNAL_ORIGINATOR_ENV` in `exec.py:16`.
const INTERNAL_ORIGINATOR_ENV: &str = "CODEX_INTERNAL_ORIGINATOR_OVERRIDE";

/// Originator marker. Python uses `codex_sdk_py`; we use `codex_sdk_rs` so
/// upstream can distinguish Rust callers.
const RUST_SDK_ORIGINATOR: &str = "codex_sdk_rs";

fn bool_lit(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn sandbox_mode_value(mode: super::SandboxMode) -> &'static str {
    match mode {
        super::SandboxMode::ReadOnly => "read-only",
        super::SandboxMode::WorkspaceWrite => "workspace-write",
        super::SandboxMode::DangerFullAccess => "danger-full-access",
    }
}

fn reasoning_effort_value(effort: super::ModelReasoningEffort) -> &'static str {
    match effort {
        super::ModelReasoningEffort::Minimal => "minimal",
        super::ModelReasoningEffort::Low => "low",
        super::ModelReasoningEffort::Medium => "medium",
        super::ModelReasoningEffort::High => "high",
    }
}

fn approval_mode_value(mode: super::ApprovalMode) -> &'static str {
    match mode {
        super::ApprovalMode::Never => "never",
        super::ApprovalMode::OnRequest => "on-request",
        super::ApprovalMode::OnFailure => "on-failure",
        super::ApprovalMode::Untrusted => "untrusted",
    }
}
