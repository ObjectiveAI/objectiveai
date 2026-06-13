use std::pin::Pin;

use envconfig::Envconfig;
use futures::Stream;
use objectiveai_sdk::cli::command::{CommandRequest, ResponseItem, parse_request};

use crate::context::Context;
use crate::error::Error;

/// Windows-only: clear `HANDLE_FLAG_INHERIT` on this process's
/// stdin/stdout/stderr handles. Called at the top of the instance
/// subprocess fast-path so plugin spawns (and any other later
/// `Stdio::piped()` spawn that triggers `bInheritHandles=TRUE`)
/// don't leak this process's stdio write ends to those children.
///
/// Without this, on Windows a plugin spawned by the instance
/// subprocess inherits the instance's stdout/stderr write ends.
/// The plugin keeps those handles open for its whole lifetime
/// (e.g. an `axum::serve` that runs forever). When the instance
/// exits, the cli outer's reads of the instance's stdout/stderr
/// don't see EOF — the plugin is still holding the write ends —
/// and the cli outer hangs forever waiting for an EOF that never
/// arrives.
#[cfg(windows)]
pub fn clear_stdio_inheritance() {
    use windows_sys::Win32::Foundation::{HANDLE_FLAG_INHERIT, SetHandleInformation};
    use windows_sys::Win32::System::Console::{
        GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE,
    };
    // SAFETY: GetStdHandle is sound to call from any thread; the
    // returned HANDLE is process-global and survives. SetHandleInformation
    // mutates only the flags on the HANDLE, never the underlying
    // file/pipe. Failure is best-effort (e.g. handle was already
    // INVALID_HANDLE_VALUE) and silently ignored — clearing the
    // inheritance flag is an optimization for child spawns, not a
    // correctness requirement for our own stdio reads/writes.
    unsafe {
        for std_id in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
            let h = GetStdHandle(std_id);
            // GetStdHandle returns INVALID_HANDLE_VALUE on error and
            // 0 / NULL when the stream isn't attached; skip both.
            if !h.is_null() && h as isize != -1 {
                let _ = SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

#[derive(Envconfig)]
struct EnvConfigBuilder {
    #[envconfig(from = "CONFIG_SET_FORBIDDEN")]
    config_set_forbidden: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_DIR")]
    objectiveai_dir: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_STATE")]
    objectiveai_state: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_NAME")]
    commit_author_name: Option<String>,
    #[envconfig(from = "COMMIT_AUTHOR_EMAIL")]
    commit_author_email: Option<String>,
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
    #[envconfig(from = "OBJECTIVEAI_PLUGIN_OWNER")]
    plugin_owner: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_PLUGIN_REPOSITORY")]
    plugin_repository: Option<String>,
    #[envconfig(from = "OBJECTIVEAI_PLUGIN_VERSION")]
    plugin_version: Option<String>,
}

impl EnvConfigBuilder {
    pub fn build(self) -> ConfigBuilder {
        fn parse_bool(s: &str) -> bool {
            let v = s.trim();
            !v.is_empty() && v != "0" && !v.eq_ignore_ascii_case("false")
        }
        ConfigBuilder {
            config_set_forbidden: self.config_set_forbidden.map(|s| parse_bool(&s)),
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            agent_instance_hierarchy: self.agent_instance_hierarchy,
            agent_id: self.agent_id,
            agent_full_id: self.agent_full_id,
            agent_remote: self.agent_remote,
            response_id: self.response_id,
            response_ids: self.response_ids,
            mcp_session_id: self.mcp_session_id,
            plugin_owner: self.plugin_owner,
            plugin_repository: self.plugin_repository,
            plugin_version: self.plugin_version,
        }
    }
}

#[derive(Default)]
pub struct ConfigBuilder {
    pub config_set_forbidden: Option<bool>,
    pub objectiveai_dir: Option<String>,
    pub objectiveai_state: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
    pub agent_instance_hierarchy: Option<String>,
    pub agent_id: Option<String>,
    pub agent_full_id: Option<String>,
    pub agent_remote: Option<String>,
    pub response_id: Option<String>,
    pub response_ids: Option<String>,
    pub mcp_session_id: Option<String>,
    pub plugin_owner: Option<String>,
    pub plugin_repository: Option<String>,
    pub plugin_version: Option<String>,
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
            objectiveai_dir: self.objectiveai_dir,
            objectiveai_state: self.objectiveai_state,
            commit_author_name: self.commit_author_name,
            commit_author_email: self.commit_author_email,
            agent_instance_hierarchy: self
                .agent_instance_hierarchy
                .unwrap_or_else(|| "cli".to_string()),
            agent_id: self.agent_id,
            agent_full_id: self.agent_full_id,
            agent_remote: self.agent_remote,
            response_id: self.response_id,
            response_ids: self.response_ids,
            mcp_session_id: self.mcp_session_id,
            plugin_owner: self.plugin_owner,
            plugin_repository: self.plugin_repository,
            plugin_version: self.plugin_version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_set_forbidden: bool,
    /// Layout root override (`OBJECTIVEAI_DIR`); None = ~/.objectiveai.
    pub objectiveai_dir: Option<String>,
    /// State name (`OBJECTIVEAI_STATE`); None = "default".
    pub objectiveai_state: Option<String>,
    pub commit_author_name: Option<String>,
    pub commit_author_email: Option<String>,
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
    /// Current response id from `OBJECTIVEAI_RESPONSE_ID` / the
    /// `X-OBJECTIVEAI-RESPONSE-ID` reverse-attach header. Propagated
    /// onto spawned plugins so plugin-side code can stamp it on
    /// outbound calls.
    pub response_id: Option<String>,
    /// Comma-separated response id chain from
    /// `OBJECTIVEAI_RESPONSE_IDS` / `X-OBJECTIVEAI-RESPONSE-IDS`.
    /// Propagated onto spawned plugins.
    pub response_ids: Option<String>,
    pub mcp_session_id: Option<String>,
    /// Plugin coordinate (`OBJECTIVEAI_PLUGIN_OWNER` / `_REPOSITORY` /
    /// `_VERSION`) of the plugin a command is running on behalf of.
    /// Set by `plugins run` on the config used to launch nested
    /// (plugin-originated) commands; assembled into
    /// [`crate::context::Context::plugin`] at startup.
    pub plugin_owner: Option<String>,
    pub plugin_repository: Option<String>,
    pub plugin_version: Option<String>,
}

/// What [`run`] yields, mirroring the two SDK root dispatch entry
/// points. The arm is chosen by whether the parsed request carries an
/// output transform (`RequestBase::transform`):
///
/// - [`RunStream::Execute`]: no transform — the typed root
///   [`ResponseItem`] stream (identical to what `run` used to return).
/// - [`RunStream::ExecuteTransform`]: a transform is set — each item is
///   the transformed JSON output (`serde_json::Value`).
pub enum RunStream {
    Execute(Pin<Box<dyn Stream<Item = Result<ResponseItem, Error>> + Send>>),
    ExecuteTransform(Pin<Box<dyn Stream<Item = Result<serde_json::Value, Error>> + Send>>),
}

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

/// Run the CLI command tree.
///
/// Clap-parse argv against the SDK's top-level command surface;
/// `TryFrom` it into [`Request`]; resolve [`Context`] (caller-
/// supplied or built from env); dispatch through the in-process
/// [`crate::executor::CliCommandExecutor`] via the SDK root
/// `execute` / `execute_transform` (the latter when the request
/// carries an output transform). Returns a [`RunStream`] whose
/// variant reflects that choice.
///
/// The `instance` subprocess subcommand is **not** handled here
/// — it has its own entry point at [`crate::instance::run`]
/// because its wire shape (`InstanceEmission`) differs from
/// [`ResponseItem`]. `main.rs` routes argv[1] == "instance" to
/// that entry directly and writes its own ndjson; ordinary
/// command flows go through this function.
///
/// Pre-dispatch failures (clap parse error, arg-conversion error,
/// context build) and child-function errors propagate as the outer
/// `Err`. On success the caller consumes the stream.
pub async fn run(
    args: Vec<String>,
    ctx: Option<Context>,
) -> Result<RunStream, Error> {
    // Windows: clear the inheritance flag on this process's
    // stdin/stdout/stderr handles. They were marked inheritable by
    // whoever spawned us via `Stdio::piped()` — necessary so we
    // inherit them at all — but if we leave the flag set, every
    // grandchild process we (or any of our descendants) spawn with
    // `Stdio::piped()` (which sets `bInheritHandles=TRUE`) inherits
    // OUR stdio handles in addition to its own new pipes. That
    // grandchild (e.g. a plugin RMCP server living for the whole
    // agent completion) then holds our stdio write ends open even
    // after we exit, leaving our parent's reads of our stdout/stderr
    // hanging forever instead of EOF'ing.
    //
    // Applies to BOTH branches:
    //   - The `instance` branch (called from a parent cli) so plugin
    //     grandchildren don't inherit the instance's stdio.
    //   - The non-instance branch (the outer cli, called from a test
    //     harness or another shell) so the instance subprocess
    //     doesn't inherit the outer cli's stdio — otherwise an
    //     orphan plugin grandchild keeps the outer cli's stdout
    //     pipe alive after the instance dies, and the harness
    //     hangs waiting for stdout EOF.
    //
    // Clearing the flag is a no-op for our own use of std{in,out,err}
    // — we keep using them normally. It only affects what gets
    // propagated on subsequent `CreateProcessW` calls.
    #[cfg(windows)]
    clear_stdio_inheritance();

    // `Context::new` is synchronous and IO-free; the API client,
    // viewer client, and db pool connect lazily on first use via
    // `ctx.{api,viewer,db}_client()`. No explicit setup call needed
    // here.
    let ctx = match ctx {
        Some(c) => c,
        None => Context::new(load_config()),
    };

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

    // Drive the request through the in-process executor, picking the
    // SDK root dispatch entry by whether the request carries an output
    // transform. The executor applies the transform / token / timeout
    // adapters; `execute_transform` additionally sets the transform on
    // the leaf request and yields the post-transform JSON, whereas
    // `execute` yields the typed root items.
    let transform = request.request_base().transform();
    let executor = crate::executor::CliCommandExecutor::new(ctx);
    match transform {
        Some(transform) => {
            let stream =
                objectiveai_sdk::cli::command::execute_transform(&executor, request, transform, None)
                    .await?;
            Ok(RunStream::ExecuteTransform(stream))
        }
        None => {
            let stream = objectiveai_sdk::cli::command::execute(&executor, request, None).await?;
            Ok(RunStream::Execute(stream))
        }
    }
}
