//! `plugins install github` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub owner: String,
    pub repository: String,
    pub commit_sha: Option<String>,
    pub allow_untrusted: bool,
    pub jq: Option<String>,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "plugins".to_string(),
            "install".to_string(),
            "github".to_string(),
            "--owner".to_string(),
            self.owner.clone(),
            "--repository".to_string(),
            self.repository.clone(),
        ];
        if let Some(sha) = &self.commit_sha {
            argv.push("--commit-sha".to_string());
            argv.push(sha.clone());
        }
        if self.allow_untrusted {
            argv.push("--allow-untrusted".to_string());
        }
        if let Some(jq) = &self.jq {
            argv.push("--jq".to_string());
            argv.push(jq.clone());
        }
        argv
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Response {
    pub installed: bool,
}

#[derive(clap::Args)]
pub struct Args {
    /// GitHub repository owner.
    #[arg(long)]
    pub owner: String,
    /// GitHub repository name.
    #[arg(long)]
    pub repository: String,
    /// Pin install to a specific commit.
    #[arg(long)]
    pub commit_sha: Option<String>,
    /// Permit installs from untrusted sources.
    #[arg(long)]
    pub allow_untrusted: bool,
    /// jq filter applied to the JSON output.
    #[arg(long)]
    pub jq: Option<String>,
}

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Command {
    #[command(flatten)]
    pub args: Args,
    #[command(subcommand)]
    pub schema: Option<Schema>,
}

#[derive(clap::Subcommand)]
pub enum Schema {
    /// Emit the JSON Schema for this leaf's `Request` type and exit.
    RequestSchema(request_schema::Args),
    /// Emit the JSON Schema for this leaf's `Response` type and exit.
    ResponseSchema(response_schema::Args),
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "install", "github", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["plugins", "install", "github", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;
}
