//! `laboratories config addresses add` — async handler stub.

use crate::cli::command::CommandRequest;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.config.addresses.add.Request")]
pub struct Request {
    pub path_type: Path,
    #[serde(flatten)]
    pub base: crate::cli::command::RequestBase,
    /// The daemon `http://` address the laboratory host should connect to.
    pub key: String,
    /// The signature to present at that address. Empty ⇒ dial
    /// unauthenticated (the address's daemon must be open). Always
    /// sent explicitly — NO serde default: empty-string schema
    /// defaults are banned (the json-schema builder asserts it), and
    /// the CLI fills `""` itself when `--value` is omitted.
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.laboratories.config.addresses.add.Path")]
pub enum Path {
    #[serde(rename = "laboratories/config/addresses/add")]
    LaboratoriesConfigAddressesAdd,
}

impl CommandRequest for Request {
    fn request_base(&self) -> &crate::cli::command::RequestBase {
        &self.base
    }

    fn request_base_mut(&mut self) -> Option<&mut crate::cli::command::RequestBase> {
        Some(&mut self.base)
    }
}

pub type Response = crate::cli::command::Ok;

#[derive(clap::Args)]
#[command(group(clap::ArgGroup::new("key_required").required(true).args(["key"])))]
pub struct Args {
    #[command(flatten)]
    pub base: crate::cli::command::RequestBaseArgs,
    /// Entry key (a daemon `http://` address).
    #[arg(long)]
    pub key: Option<String>,
    /// Entry value (the signature to present at that address).
    /// Omitted ⇒ dial unauthenticated.
    #[arg(long)]
    pub value: Option<String>,
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
        Ok(Self {
            base: args.base.into(), path_type: Path::LaboratoriesConfigAddressesAdd,
            key: args.key.ok_or_else(|| {
                crate::cli::command::FromArgsError::path_parse(
                    "key",
                    "--key is required".to_string(),
                )
            })?,
            // Optional: an address without a signature dials
            // unauthenticated.
            value: args.value.unwrap_or_default(),
        })
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<Response, E::Error> {
    executor.execute_one(request, identity).await
}

pub mod request_schema;

pub mod response_schema;

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    _transform: crate::cli::command::Transform,

        identity: Option<&crate::identity::Identity>,
    ) -> Result<serde_json::Value, E::Error> {
    let resp: Response = executor.execute_one(request, identity).await?;
    Ok(serde_json::to_value(resp).expect("Response serializes"))
}

/// One `/listen` broadcast run of `laboratories config addresses add`: the actual
/// [`Request`], the producer's
/// [`Identity`](crate::identity::Identity), and the
/// unary response future. See [`crate::daemon::command_listener`].
#[cfg(all(feature = "cli", feature = "daemon"))]
pub struct ListenerExecution {
    pub request: Request,
    pub identity: crate::identity::Identity,
    pub response: crate::daemon::command_listener::UnaryResponse<Response>,
}
