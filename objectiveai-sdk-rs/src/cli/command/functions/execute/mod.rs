pub mod standard;
pub mod swiss_system;

/// CLI-surface form for the `--function*` argument family: either a
/// fully resolved inline-or-remote spec (the JSON object form that
/// lands on `--function-inline`, or the docker-style remote-path
/// string on `--function`), the path to a JSON file (`--function-file`), or a Python harness
/// — inline (`--function-python-inline`) or file
/// (`--function-python-file`) — that produces the inline-or-remote
/// JSON at handler time.
///
/// Untagged so existing on-disk JSON for `Resolved`
/// round-trips byte-identically; `File`/`PythonInline`/`PythonFile`
/// are new variants that only appear on the cli wire side.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.FunctionSpec")]
pub enum FunctionSpec {
    #[schemars(title = "Resolved")]
    Resolved(crate::functions::FullInlineFunctionOrRemoteCommitOptional),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

/// CLI-surface form for the `--profile*` argument family — same
/// shape as [`FunctionSpec`] but wrapping the profile inline-or-remote
/// enum. See [`FunctionSpec`] for the variant semantics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.ProfileSpec")]
pub enum ProfileSpec {
    #[schemars(title = "Resolved")]
    Resolved(crate::functions::InlineProfileOrRemoteCommitOptional),
    #[schemars(title = "File")]
    File(std::path::PathBuf),
    #[schemars(title = "PythonInline")]
    PythonInline(String),
    #[schemars(title = "PythonFile")]
    PythonFile(std::path::PathBuf),
}

impl FunctionSpec {
    /// Emit this spec back into the argv pair the cli's clap parser
    /// will reconstruct from. Used by [`crate::cli::command::CommandRequest::into_command`].
    pub fn push_flags(&self, out: &mut Vec<String>) {
        use crate::cli::command::path_ref::remote_path_to_arg_string;
        use crate::functions::FullInlineFunctionOrRemoteCommitOptional;
        match self {
            FunctionSpec::Resolved(FullInlineFunctionOrRemoteCommitOptional::Remote(p)) => {
                out.push("--function".to_string());
                out.push(remote_path_to_arg_string(p));
            }
            FunctionSpec::Resolved(inline @ FullInlineFunctionOrRemoteCommitOptional::Inline(_)) => {
                out.push("--function-inline".to_string());
                out.push(serde_json::to_string(inline).expect("function serializes"));
            }
            FunctionSpec::File(p) => {
                out.push("--function-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            FunctionSpec::PythonInline(code) => {
                out.push("--function-python-inline".to_string());
                out.push(code.clone());
            }
            FunctionSpec::PythonFile(p) => {
                out.push("--function-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

impl ProfileSpec {
    /// Emit this spec back into the argv pair the cli's clap parser
    /// will reconstruct from. Used by [`crate::cli::command::CommandRequest::into_command`].
    pub fn push_flags(&self, out: &mut Vec<String>) {
        use crate::cli::command::path_ref::remote_path_to_arg_string;
        use crate::functions::InlineProfileOrRemoteCommitOptional;
        match self {
            ProfileSpec::Resolved(InlineProfileOrRemoteCommitOptional::Remote(p)) => {
                out.push("--profile".to_string());
                out.push(remote_path_to_arg_string(p));
            }
            ProfileSpec::Resolved(inline @ InlineProfileOrRemoteCommitOptional::Inline(_)) => {
                out.push("--profile-inline".to_string());
                out.push(serde_json::to_string(inline).expect("profile serializes"));
            }
            ProfileSpec::File(p) => {
                out.push("--profile-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
            ProfileSpec::PythonInline(code) => {
                out.push("--profile-python-inline".to_string());
                out.push(code.clone());
            }
            ProfileSpec::PythonFile(p) => {
                out.push("--profile-python-file".to_string());
                out.push(p.to_string_lossy().into_owned());
            }
        }
    }
}

/// Exactly-one-of `--function | --function-inline | --function-file |
/// --function-python-inline | --function-python-file`. Lives on its
/// own sub-struct + group so the `required = true, multiple = false`
/// enforcement is scoped to these five fields (clap derive's
/// default-group rule would otherwise pull every outer field into
/// the same group). The group id is `function_group` rather than
/// `function` to avoid colliding with the bare `--function` flag's
/// own arg name.
#[derive(clap::Args)]
#[group(id = "function_group", required = true, multiple = false)]
pub struct FunctionArgs {
    /// Remote-path ref in docker-style key=value form
    /// (e.g. `remote=mock,name=foo`).
    #[arg(long, group = "function_group")]
    pub function: Option<String>,
    /// Inline JSON function definition (inline or remote-object shape).
    #[arg(long, group = "function_group")]
    pub function_inline: Option<String>,
    /// Path to a JSON file containing the function definition.
    #[arg(long, group = "function_group")]
    pub function_file: Option<std::path::PathBuf>,
    /// Inline Python that returns the function definition.
    #[arg(long, group = "function_group")]
    pub function_python_inline: Option<String>,
    /// Path to a Python file that returns the function definition.
    #[arg(long, group = "function_group")]
    pub function_python_file: Option<std::path::PathBuf>,
}

/// Mirror of [`FunctionArgs`] for `--profile*`. See that struct for
/// the variant semantics + group-id rationale.
#[derive(clap::Args)]
#[group(id = "profile_group", required = true, multiple = false)]
pub struct ProfileArgs {
    /// Remote-path ref in docker-style key=value form
    /// (e.g. `remote=mock,name=foo`).
    #[arg(long, group = "profile_group")]
    pub profile: Option<String>,
    /// Inline JSON profile definition (inline or remote-object shape).
    #[arg(long, group = "profile_group")]
    pub profile_inline: Option<String>,
    /// Path to a JSON file containing the profile definition.
    #[arg(long, group = "profile_group")]
    pub profile_file: Option<std::path::PathBuf>,
    /// Inline Python that returns the profile definition.
    #[arg(long, group = "profile_group")]
    pub profile_python_inline: Option<String>,
    /// Path to a Python file that returns the profile definition.
    #[arg(long, group = "profile_group")]
    pub profile_python_file: Option<std::path::PathBuf>,
}

impl TryFrom<FunctionArgs> for FunctionSpec {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: FunctionArgs) -> Result<Self, Self::Error> {
        use crate::functions::FullInlineFunctionOrRemoteCommitOptional;
        if let Some(s) = args.function {
            let path: crate::RemotePathCommitOptional = s
                .parse()
                .map_err(|e| crate::cli::command::FromArgsError::path_parse("function", e))?;
            Ok(FunctionSpec::Resolved(
                FullInlineFunctionOrRemoteCommitOptional::Remote(path),
            ))
        } else if let Some(s) = args.function_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de)
                .map_err(|e| crate::cli::command::FromArgsError::json("function_inline", e))?;
            Ok(FunctionSpec::Resolved(v))
        } else if let Some(p) = args.function_file {
            Ok(FunctionSpec::File(p))
        } else if let Some(s) = args.function_python_inline {
            Ok(FunctionSpec::PythonInline(s))
        } else {
            Ok(FunctionSpec::PythonFile(args.function_python_file.unwrap()))
        }
    }
}

impl TryFrom<ProfileArgs> for ProfileSpec {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(args: ProfileArgs) -> Result<Self, Self::Error> {
        use crate::functions::InlineProfileOrRemoteCommitOptional;
        if let Some(s) = args.profile {
            let path: crate::RemotePathCommitOptional = s
                .parse()
                .map_err(|e| crate::cli::command::FromArgsError::path_parse("profile", e))?;
            Ok(ProfileSpec::Resolved(
                InlineProfileOrRemoteCommitOptional::Remote(path),
            ))
        } else if let Some(s) = args.profile_inline {
            let mut de = serde_json::Deserializer::from_str(&s);
            let v = serde_path_to_error::deserialize(&mut de)
                .map_err(|e| crate::cli::command::FromArgsError::json("profile_inline", e))?;
            Ok(ProfileSpec::Resolved(v))
        } else if let Some(p) = args.profile_file {
            Ok(ProfileSpec::File(p))
        } else if let Some(s) = args.profile_python_inline {
            Ok(ProfileSpec::PythonInline(s))
        } else {
            Ok(ProfileSpec::PythonFile(args.profile_python_file.unwrap()))
        }
    }
}

#[derive(clap::Subcommand)]
pub enum Command {
    Standard(standard::Command),
    SwissSystem(swiss_system::Command),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[schemars(rename = "cli.command.functions.execute.Request")]
pub enum Request {
    #[schemars(title = "Standard")]
    Standard(standard::Request),
    #[schemars(title = "StandardRequestSchema")]
    StandardRequestSchema(standard::request_schema::Request),
    #[schemars(title = "StandardResponseSchema")]
    StandardResponseSchema(standard::response_schema::Request),
    #[schemars(title = "SwissSystem")]
    SwissSystem(swiss_system::Request),
    #[schemars(title = "SwissSystemRequestSchema")]
    SwissSystemRequestSchema(swiss_system::request_schema::Request),
    #[schemars(title = "SwissSystemResponseSchema")]
    SwissSystemResponseSchema(swiss_system::response_schema::Request),
}

// Exempt from json-schema coverage: tier aggregate (see the root
// `ResponseItem` in command.rs - TS7056).
#[objectiveai_sdk_macros::json_schema_ignore]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[schemars(rename = "cli.command.functions.execute.ResponseItem")]
#[serde(untagged)]
pub enum ResponseItem {
    #[schemars(title = "Standard")]
    Standard(standard::ResponseItem),
    #[schemars(title = "StandardRequestSchema")]
    StandardRequestSchema(standard::request_schema::Response),
    #[schemars(title = "StandardResponseSchema")]
    StandardResponseSchema(standard::response_schema::Response),
    #[schemars(title = "SwissSystem")]
    SwissSystem(swiss_system::ResponseItem),
    #[schemars(title = "SwissSystemRequestSchema")]
    SwissSystemRequestSchema(swiss_system::request_schema::Response),
    #[schemars(title = "SwissSystemResponseSchema")]
    SwissSystemResponseSchema(swiss_system::response_schema::Response),
}

#[cfg(feature = "mcp")]
impl crate::cli::command::CommandResponse for ResponseItem {
    fn into_mcp(self) -> crate::cli::command::McpResponseItem {
        match self {
            ResponseItem::Standard(v) => v.into_mcp(),
            ResponseItem::StandardRequestSchema(v) => v.into_mcp(),
            ResponseItem::StandardResponseSchema(v) => v.into_mcp(),
            ResponseItem::SwissSystem(v) => v.into_mcp(),
            ResponseItem::SwissSystemRequestSchema(v) => v.into_mcp(),
            ResponseItem::SwissSystemResponseSchema(v) => v.into_mcp(),
        }
    }
}

impl TryFrom<Command> for Request {
    type Error = crate::cli::command::FromArgsError;
    fn try_from(command: Command) -> Result<Self, Self::Error> {
        match command {
            Command::Standard(cmd) => match cmd.schema {
                None => Ok(Request::Standard(standard::Request::try_from(cmd.args)?)),
                Some(standard::Schema::RequestSchema(args)) =>
                    Ok(Request::StandardRequestSchema(standard::request_schema::Request::try_from(args)?)),
                Some(standard::Schema::ResponseSchema(args)) =>
                    Ok(Request::StandardResponseSchema(standard::response_schema::Request::try_from(args)?)),
            },
            Command::SwissSystem(cmd) => match cmd.schema {
                None => Ok(Request::SwissSystem(swiss_system::Request::try_from(cmd.args)?)),
                Some(swiss_system::Schema::RequestSchema(args)) =>
                    Ok(Request::SwissSystemRequestSchema(swiss_system::request_schema::Request::try_from(args)?)),
                Some(swiss_system::Schema::ResponseSchema(args)) =>
                    Ok(Request::SwissSystemResponseSchema(swiss_system::response_schema::Request::try_from(args)?)),
            },
        }
    }
}

impl crate::cli::command::CommandRequest for Request {
    fn into_command(&self) -> Vec<String> {
        match self {
            Request::Standard(inner) => inner.into_command(),
            Request::StandardRequestSchema(inner) => inner.into_command(),
            Request::StandardResponseSchema(inner) => inner.into_command(),
            Request::SwissSystem(inner) => inner.into_command(),
            Request::SwissSystemRequestSchema(inner) => inner.into_command(),
            Request::SwissSystemResponseSchema(inner) => inner.into_command(),
        }
    }
}

#[cfg(feature = "cli-executor")]
pub async fn execute<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>>,
    E::Error,
> {
    use futures::StreamExt;
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<ResponseItem, E::Error>> + Send>> =
        match request {
            Request::Standard(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = standard::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::Standard)))
                } else {
                    let value = standard::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::Standard(standard::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::StandardRequestSchema(req) => {
                let value = standard::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::StandardRequestSchema(value),
                )))
            }
            Request::StandardResponseSchema(req) => {
                let value = standard::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::StandardResponseSchema(value),
                )))
            }
            Request::SwissSystem(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = swiss_system::execute_streaming(executor, req, agent_arguments).await?;
                    Box::pin(inner.map(|r| r.map(ResponseItem::SwissSystem)))
                } else {
                    let value = swiss_system::execute(executor, req, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(
                        ResponseItem::SwissSystem(swiss_system::ResponseItem::Id(value)),
                    )))
                }
            }
            Request::SwissSystemRequestSchema(req) => {
                let value = swiss_system::request_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SwissSystemRequestSchema(value),
                )))
            }
            Request::SwissSystemResponseSchema(req) => {
                let value = swiss_system::response_schema::execute(executor, req, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(
                    ResponseItem::SwissSystemResponseSchema(value),
                )))
            }
        };
    Ok(stream)
}

#[cfg(feature = "cli-executor")]
pub async fn execute_transform<E: crate::cli::command::CommandExecutor>(
    executor: &E,
    request: Request,
    transform: crate::cli::command::Transform,

        agent_arguments: Option<&crate::cli::command::AgentArguments>,
    ) -> Result<
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>>,
    E::Error,
> {
    let stream: std::pin::Pin<Box<dyn futures::Stream<Item = Result<serde_json::Value, E::Error>> + Send>> =
        match request {
            Request::Standard(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = standard::execute_streaming_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = standard::execute_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::StandardRequestSchema(req) => {
                let value = standard::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::StandardResponseSchema(req) => {
                let value = standard::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SwissSystem(req) => {
                let want_streaming = req
                    .dangerous_advanced
                    .as_ref()
                    .and_then(|a| a.stream)
                    .unwrap_or(false);
                if want_streaming {
                    let inner = swiss_system::execute_streaming_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(inner)
                } else {
                    let value = swiss_system::execute_transform(executor, req, transform, agent_arguments).await?;
                    Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
                }
            }
            Request::SwissSystemRequestSchema(req) => {
                let value = swiss_system::request_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
            Request::SwissSystemResponseSchema(req) => {
                let value = swiss_system::response_schema::execute_transform(executor, req, transform, agent_arguments).await?;
                Box::pin(crate::cli::command::StreamOnce::new(Ok(value)))
            }
        };
    Ok(stream)
}
