use std::pin::Pin;

use envconfig::Envconfig;
use futures::{Stream, StreamExt};
use objectiveai_sdk::cli::command::{ResponseItem, parse_request};

use crate::context::Context;
use crate::error::Error;
use crate::instance::InstanceEmission;

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "CONFIG_SET_FORBIDDEN")]
    config_set_forbidden: Option<String>,
    #[envconfig(from = "CONFIG_BASE_DIR")]
    config_base_dir: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
    #[envconfig(from = "GITHUB_AUTHORIZATION")]
    github_authorization: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY")]
    agent_instance_hierarchy: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AGENT_ID")]
    agent_id: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AGENT_FULL_ID")]
    agent_full_id: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_AGENT_REMOTE")]
    agent_remote: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_RESPONSE_ID")]
    response_id: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_RESPONSE_IDS")]
    response_ids: Option<String>,
    #[envconfig(from = "MCP_SESSION_ID")]
    mcp_session_id: Option<String>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        fn parse_bool(s: &str) -> bool {
            let v = s.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        }
        ConfigBuilder {
            config_set_forbidden: self.config_set_forbidden.map(|s| parse_bool(&s)),
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            github_authorization: self.github_authorization,
            agent_instance_hierarchy: self.agent_instance_hierarchy,
            agent_id: self.agent_id,
            agent_full_id: self.agent_full_id,
            agent_remote: self.agent_remote,
            response_id: self.response_id,
            response_ids: self.response_ids,
            mcp_session_id: self.mcp_session_id,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub config_set_forbidden: Option<bool>,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub github_authorization: Option<String>,
    pub agent_instance_hierarchy: Option<String>,
    pub agent_id: Option<String>,
    pub agent_full_id: Option<String>,
    pub agent_remote: Option<String>,
    pub response_id: Option<String>,
    pub response_ids: Option<String>,
    pub mcp_session_id: Option<String>,
}

impl Envconfig for ConfigBuilder {
    #[allow(deprecated)]
    fn init() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init().map(|e| e.build())
    }

    fn init_from_env() -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_env().map(|e| e.build())
    }

    fn init_from_hashmap(
        hashmap: &std::collections::HashMap<String, String>,
    ) -> Result<Self, envconfig::Error> {
        EnvConfigBuilder::init_from_hashmap(hashmap).map(|e| e.build())
    }
}

impl ConfigBuilder {
    pub fn build(self) -> Config {
        Config {
            config_set_forbidden: self.config_set_forbidden.unwrap_or(false),
            config_base_dir: self.config_base_dir,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            github_authorization: self.github_authorization,
            agent_instance_hierarchy: self
                .agent_instance_hierarchy
                .unwrap_or_else(|| "cli".to_string()),
            agent_id: self.agent_id,
            agent_full_id: self.agent_full_id,
            agent_remote: self.agent_remote,
            mcp_session_id: self.mcp_session_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_set_forbidden: bool,
    pub config_base_dir: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub github_authorization: Option<String>,
    pub agent_instance_hierarchy: String,
    pub agent_id: Option<String>,
    /// WF-level agent identity from `OBJECTIVEAI_AGENT_FULL_ID` / the
    /// `X-OBJECTIVEAI-AGENT-FULL-ID` reverse-attach header. Propagated
    /// onto spawned plugin subprocesses by the conduit so plugin-side
    /// code can stamp it on outbound calls.
    pub agent_full_id: Option<String>,
    /// JSON-encoded `RemotePath` from `OBJECTIVEAI_AGENT_REMOTE` /
    /// the `X-OBJECTIVEAI-AGENT-REMOTE` reverse-attach header. Empty
    /// when the WF was inline. Propagated onto spawned plugins.
    pub agent_remote: Option<String>,
    pub mcp_session_id: Option<String>,
}

/// One typed item yielded by [`run`]. The two variants reflect the
/// two dispatch paths: the bare-naked command tree (the SDK's typed
/// `ResponseItem` aggregator) or the instance subprocess runtime (its
/// own typed [`InstanceEmission`] enum).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum RunItem {
    Command(ResponseItem),
    Instance(InstanceEmission),
}

#[cfg(feature = "mcp")]
impl objectiveai_sdk::cli::command::CommandResponse for RunItem {
    fn into_mcp(
        self,
    ) -> objectiveai_sdk::cli::command::McpResponseItem {
        match self {
            RunItem::Command(item) => item.into_mcp(),
            RunItem::Instance(emission) => {
                objectiveai_sdk::cli::command::McpResponseItem::JSONL(
                    serde_json::to_value(emission)
                        .unwrap_or(serde_json::Value::Null),
                )
            }
        }
    }
}

pub type RunStream = Pin<Box<dyn Stream<Item = Result<RunItem, Error>> + Send>>;

/// Build the top-level CLI config from the process environment.
pub fn load_config() -> Config {
    ConfigBuilder::init_from_env().unwrap_or_default().build()
}

/// Did clap exit with one of the "successful informational output"
/// variants? `--help`, `--version`, or a missing-subcommand bail.
pub fn is_informational(e: &clap::Error) -> bool {
    use clap::error::ErrorKind;
    matches!(
        e.kind(),
        ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
    )
}

/// Run the CLI.
///
/// Step 1: if argv[1] is `"instance"`, fast-path into the
/// FD-handshake instance subcommand — no clap, no [`Context`],
/// no SDK [`Request`]. The instance handler returns its own typed
/// [`InstanceEmission`] stream which we wrap as
/// [`RunItem::Instance`].
///
/// Step 2: otherwise, clap-parse argv against the SDK's top-level
/// [`SdkCommand`]; `TryFrom` it into [`Request`]; resolve
/// [`Context`] (caller-supplied or built from env); dispatch through
/// `crate::command::command::execute`. Each yielded
/// [`ResponseItem`] is wrapped as [`RunItem::Command`].
///
/// Pre-dispatch failures (clap parse error, arg-conversion error,
/// context build) and child-function errors propagate as the outer
/// `Err`. On success the caller consumes the stream.
pub async fn run(
    args: Vec<String>,
    ctx: Option<Context>,
) -> Result<RunStream, Error> {
    if args.get(1).map(String::as_str) == Some("instance") {
        let inst = crate::instance::run().await?;
        return Ok(Box::pin(inst.map(|r| r.map(RunItem::Instance))));
    }

    // `args[0]` is the program name however the binary was invoked —
    // bare name from PATH, full path from a test harness or a
    // `current_exe()` self-respawn — never part of the command.
    // Strip it unconditionally; `parse_request` prepends its own
    // canonical bin name. (Matching on the literal `"objectiveai"`
    // inside `parse_request` is NOT enough: a full-path argv[0] like
    // `C:\...\objectiveai-cli.exe` would be parsed as a subcommand.)
    let request = parse_request(args.get(1..).unwrap_or_default()).map_err(|e| match e {
        objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
        objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
    })?;

    let ctx = match ctx {
        Some(c) => c,
        None => Context::new(load_config()).await?,
    };

    // TODO(jq): if the resolved request carries a `jq` filter, extract
    // it here and apply to the returned stream before wrapping.
    let typed = crate::command::command::execute(&ctx, request).await?;
    Ok(Box::pin(typed.map(|r| r.map(RunItem::Command))))
}
