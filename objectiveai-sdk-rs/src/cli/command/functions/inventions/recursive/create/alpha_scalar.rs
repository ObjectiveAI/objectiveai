//! `functions inventions recursive create alpha-scalar` — async handler stub.

use crate::agent::InlineAgentBaseWithFallbacksOrRemoteCommitOptional;
use crate::cli::command::IntoCommand;

pub struct Request {
    pub params: RequestParams,
    pub agent: InlineAgentBaseWithFallbacksOrRemoteCommitOptional,
    pub continuation: Option<String>,
    pub seed: Option<i64>,
    pub detach: bool,
}

pub struct RequestParams {
    pub name: String,
    pub spec: String,
    pub depth: u64,
    pub min_branch_width: u64,
    pub max_branch_width: u64,
    pub min_leaf_width: u64,
    pub max_leaf_width: u64,
}

impl RequestParams {
    fn push_flags(&self, out: &mut Vec<String>) {
        out.push("--name".to_string());
        out.push(self.name.clone());
        out.push("--spec".to_string());
        out.push(self.spec.clone());
        out.push("--depth".to_string());
        out.push(self.depth.to_string());
        out.push("--min-branch-width".to_string());
        out.push(self.min_branch_width.to_string());
        out.push("--max-branch-width".to_string());
        out.push(self.max_branch_width.to_string());
        out.push("--min-leaf-width".to_string());
        out.push(self.min_leaf_width.to_string());
        out.push("--max-leaf-width".to_string());
        out.push(self.max_leaf_width.to_string());
    }
}

impl IntoCommand for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "functions".to_string(),
            "inventions".to_string(),
            "recursive".to_string(),
            "create".to_string(),
            "alpha-scalar".to_string(),
        ];
        self.params.push_flags(&mut argv);
        argv.push("--agent-inline".to_string());
        argv.push(serde_json::to_string(&self.agent).expect("agent serializes"));
        if let Some(c) = &self.continuation {
            argv.push("--continuation".to_string());
            argv.push(c.clone());
        }
        if let Some(seed) = self.seed {
            argv.push("--seed".to_string());
            argv.push(seed.to_string());
        }
        if self.detach {
            argv.push("--detach".to_string());
        }
        argv
    }
}
