//! `config functions profiles pairs favorites edit` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Request {
    pub name: String,
    pub note: Option<String>,
    pub function_commit: Option<RequestCommitChange>,
    pub profile_commit: Option<RequestCommitChange>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum RequestCommitChange {
    Set(String),
    Remove,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec![
            "config".to_string(),
            "functions".to_string(),
            "profiles".to_string(),
            "pairs".to_string(),
            "favorites".to_string(),
            "edit".to_string(),
            self.name.clone(),
        ];
        if let Some(note) = &self.note {
            argv.push("--note".to_string());
            argv.push(note.clone());
        }
        match &self.function_commit {
            Some(RequestCommitChange::Set(c)) => {
                argv.push("--function-commit".to_string());
                argv.push(c.clone());
            }
            Some(RequestCommitChange::Remove) => {
                argv.push("--remove-function-commit".to_string());
            }
            None => {}
        }
        match &self.profile_commit {
            Some(RequestCommitChange::Set(c)) => {
                argv.push("--profile-commit".to_string());
                argv.push(c.clone());
            }
            Some(RequestCommitChange::Remove) => {
                argv.push("--remove-profile-commit".to_string());
            }
            None => {}
        }
        argv
    }
}

pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
pub struct Args {
    /// Favorite name.
    pub name: String,
    /// New note (omit to leave unchanged).
    #[arg(long)]
    pub note: Option<String>,
    /// Set the pinned commit SHA on the function path.
    #[arg(long, conflicts_with = "remove_function_commit")]
    pub function_commit: Option<String>,
    /// Remove the pinned commit SHA on the function path.
    #[arg(long, conflicts_with = "function_commit")]
    pub remove_function_commit: bool,
    /// Set the pinned commit SHA on the profile path.
    #[arg(long, conflicts_with = "remove_profile_commit")]
    pub profile_commit: Option<String>,
    /// Remove the pinned commit SHA on the profile path.
    #[arg(long, conflicts_with = "profile_commit")]
    pub remove_profile_commit: bool,
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

impl TryFrom<Args> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: Args) -> Result<Self, Self::Error> {
        let function_commit = if let Some(c) = args.function_commit {
            Some(RequestCommitChange::Set(c))
        } else if args.remove_function_commit {
            Some(RequestCommitChange::Remove)
        } else {
            None
        };
        let profile_commit = if let Some(c) = args.profile_commit {
            Some(RequestCommitChange::Set(c))
        } else if args.remove_profile_commit {
            Some(RequestCommitChange::Remove)
        } else {
            None
        };
        Ok(Self {
            name: args.name,
            note: args.note,
            function_commit,
            profile_commit,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
) -> Result<Response, E::Error> {
    executor.execute_one(request).await
}

pub mod request_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["config", "functions", "profiles", "pairs", "favorites", "edit", "request-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}


pub mod response_schema {
    use crate::cli::command::CommandRequest;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
    pub struct Request {
        pub jq: Option<String>,
    }

    impl CommandRequest for Request {
        fn into_command(&self) -> Vec<String> {
            let mut argv: Vec<String> = vec!["config", "functions", "profiles", "pairs", "favorites", "edit", "response-schema"].into_iter().map(String::from).collect();
            if let Some(jq) = &self.jq {
                argv.push("--jq".to_string());
                argv.push(jq.clone());
            }
            argv
        }
    }

    pub type Response = schemars::Schema;

    impl TryFrom<Args> for Request {
        type Error = crate::cli::command::FromArgsError;
        fn try_from(args: Args) -> Result<Self, Self::Error> {
            Ok(Self { jq: args.jq })
        }
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
    ) -> Result<Response, E::Error> {
        request.jq = None;
        executor.execute_one(request).await
    }

    #[cfg(feature = "cli-executor")]
    pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
        executor: &E,
        mut request: Request,
        jq: String,
    ) -> Result<serde_json::Value, E::Error> {
        request.jq = Some(jq);
        executor.execute_one(request).await
    }
}
