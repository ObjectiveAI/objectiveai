//! `instance agents spawn` — async handler stub.

use crate::agent::completions::request::AgentCompletionCreateParams;
use crate::cli::command::IntoCommand;

pub struct Request {
    // HttpArgs (inlined, no shared types per round)
    pub api_address: Option<String>,
    pub objectiveai_authorization: Option<String>,
    pub user_agent: Option<String>,
    pub x_title: Option<String>,
    pub http_referer: Option<String>,
    pub github_authorization: Option<String>,
    pub openrouter_authorization: Option<String>,
    pub mcp_authorization: Option<String>,
    pub viewer_signature: Option<String>,
    pub viewer_address: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub objectiveai_agent_instance_hierarchy: Option<String>,
    pub objectiveai_agent_id: Option<String>,
    pub mcp_session_id: Option<String>,
    // PipeArgs (inlined)
    pub config_base_dir: Option<std::path::PathBuf>,
    pub mcp_address: Option<String>,
    pub bind_agent_instance_hierarchy: Option<String>,
    // Body — typed SDK params
    pub body: AgentCompletionCreateParams,
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["instance".to_string()];
        push_opt(&mut argv, "--api-address", &self.api_address);
        push_opt(&mut argv, "--objectiveai-authorization", &self.objectiveai_authorization);
        push_opt(&mut argv, "--user-agent", &self.user_agent);
        push_opt(&mut argv, "--x-title", &self.x_title);
        push_opt(&mut argv, "--http-referer", &self.http_referer);
        push_opt(&mut argv, "--github-authorization", &self.github_authorization);
        push_opt(&mut argv, "--openrouter-authorization", &self.openrouter_authorization);
        push_opt(&mut argv, "--mcp-authorization", &self.mcp_authorization);
        push_opt(&mut argv, "--viewer-signature", &self.viewer_signature);
        push_opt(&mut argv, "--viewer-address", &self.viewer_address);
        push_opt(&mut argv, "--commit-author-name", &self.commit_author_name);
        push_opt(&mut argv, "--commit-author-email", &self.commit_author_email);
        push_opt(
            &mut argv,
            "--objectiveai-agent-instance-hierarchy",
            &self.objectiveai_agent_instance_hierarchy,
        );
        push_opt(&mut argv, "--objectiveai-agent-id", &self.objectiveai_agent_id);
        push_opt(&mut argv, "--mcp-session-id", &self.mcp_session_id);
        if let Some(dir) = &self.config_base_dir {
            argv.push("--config-base-dir".to_string());
            argv.push(dir.to_string_lossy().into_owned());
        }
        push_opt(&mut argv, "--mcp-address", &self.mcp_address);
        push_opt(
            &mut argv,
            "--bind-agent-instance-hierarchy",
            &self.bind_agent_instance_hierarchy,
        );
        argv.extend(["agents".to_string(), "spawn".to_string()]);
        argv.push("--body".to_string());
        argv.push(serde_json::to_string(&self.body).expect("body serializes"));
        argv
    }
}

fn push_opt(argv: &mut Vec<String>, flag: &str, value: &Option<String>) {
    if let Some(v) = value {
        argv.push(flag.to_string());
        argv.push(v.clone());
    }
}
