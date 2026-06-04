//! `config functions profiles favorites edit` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.favorites.edit.Request")]
pub struct Request {
    pub path_type: Path,
    pub name: String,
    pub note: Option<String>,
    pub commit: Option<RequestCommitChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.favorites.edit.Path")]
pub enum Path {
    #[serde(rename = "config/functions/profiles/favorites/edit")]
    ConfigFunctionsProfilesFavoritesEdit,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.config.functions.profiles.favorites.edit.RequestCommitChange")]
pub enum RequestCommitChange {
    #[schemars(title = "Set")]
    Set(String),
    #[schemars(title = "Remove")]
    Remove,
}

impl CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        let mut argv = vec!["config".to_string(), "functions".to_string(), "profiles".to_string(), "favorites".to_string(), "edit".to_string(), self.name.clone()];
        if let Some(note) = &self.note {
            argv.push("--note".to_string());
            argv.push(note.clone());
        }
        match &self.commit {
            Some(RequestCommitChange::Set(c)) => {
                argv.push("--commit".to_string());
                argv.push(c.clone());
            }
            Some(RequestCommitChange::Remove) => {
                argv.push("--remove-commit".to_string());
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
    /// Set the pinned commit SHA.
    #[arg(long, conflicts_with = "remove_commit")]
    pub commit: Option<String>,
    /// Remove the pinned commit SHA.
    #[arg(long, conflicts_with = "commit")]
    pub remove_commit: bool,
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
        let commit = if let Some(c) = args.commit {
            Some(RequestCommitChange::Set(c))
        } else if args.remove_commit {
            Some(RequestCommitChange::Remove)
        } else {
            None
        };
        Ok(Self { path_type: Path::ConfigFunctionsProfilesFavoritesEdit,
            name: args.name,
            note: args.note,
            commit,
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<Response, E::Error> {
    executor.execute_one(request, agent_arguments).await
}

pub mod request_schema;


pub mod response_schema;

#[cfg(feature = "cli-executor")]
pub async fn execute_jq<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    _jq: String,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<serde_json::Value, E::Error> {
    let resp: Response = executor.execute_one(request, agent_arguments).await?;
    Ok(serde_json::to_value(resp).expect("Response serializes"))
}
