use std::pin::Pin;

use envconfig::Envconfig;
use futures::Stream;
use objectiveai_sdk::cli::command::{CommandRequest, ResponseItem, parse_request};
use objectiveai_sdk::cli::websocket_listener::ListenerEnd;

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
    #[envconfig(from = "DAEMON_ADDRESS")]
    daemon_address: Option<String>,
    #[envconfig(from = "DAEMON_PORT")]
    daemon_port: Option<u16>,
    #[envconfig(from = "DAEMON_SECRET")]
    daemon_secret: Option<String>,
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
            daemon_address: self.daemon_address,
            daemon_port: self.daemon_port,
            daemon_secret: self.daemon_secret,
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
    pub daemon_address: Option<String>,
    pub daemon_port: Option<u16>,
    pub daemon_secret: Option<String>,
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
            daemon_address: self
                .daemon_address
                .unwrap_or_else(|| "127.0.0.1".to_string()),
            daemon_port: self.daemon_port.unwrap_or(0),
            daemon_secret: self.daemon_secret,
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
    /// Bind address for the resident daemon's broadcast WebSocket
    /// server (`DAEMON_ADDRESS`); default `127.0.0.1`.
    pub daemon_address: String,
    /// Bind port for the resident daemon's broadcast WebSocket server
    /// (`DAEMON_PORT`); default `0` (OS-assigned).
    pub daemon_port: u16,
    /// Optional shared secret for the daemon's WebSocket server
    /// (`DAEMON_SECRET`). When set, every connection's first-message
    /// auth preamble must carry a valid `sha256=<hex(SHA256(secret))>`
    /// signature; when `None`, the server is open.
    pub daemon_secret: Option<String>,
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
/// [`crate::executor::DaemonCommandExecutor`] via the SDK root
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
// NOTE: explicit boxed future (not `async fn`). `run` calls the SDK
// root dispatch through `DaemonCommandExecutor`, and `tasks run` /
// `plugins run` re-enter `run` — so `run`, the executor, and the local
// command dispatch are mutually recursive. An `async fn`'s opaque
// return type can't be computed through that cycle (E0391); an explicit
// `Pin<Box<dyn Future + Send>>` return gives the recursion a known type
// to terminate on.
pub fn run(
    args: Vec<String>,
    ctx: Option<Context>,
) -> Pin<Box<dyn std::future::Future<Output = Result<RunStream, Error>> + Send>> {
    Box::pin(async move {
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
    // A top-level `--request <json>` (handled inside `parse_request` via
    // `TryFrom<Command>`) executes a JSON `CliCommandRequest` directly,
    // mutually exclusive with any command path (clap enforces it). Either
    // front door converges here on the same typed `Request`.
    let request = parse_request(args.get(1..).unwrap_or_default()).map_err(|e| match e {
        objectiveai_sdk::cli::command::ParseError::Clap(e) => Error::ClapParse(e),
        objectiveai_sdk::cli::command::ParseError::FromArgs(e) => Error::FromArgs(e),
    })?;

    // Producer tee: before executing, ensure the resident daemon is up
    // (idempotent — no respawn if already running) and open a feed of
    // this run's request + stream items into its broadcast socket. Wholly
    // best-effort: any failure yields `None` and the command runs
    // unaffected. The daemon foreground itself is skipped (see
    // `should_tee`).
    let feed = start_tee(&ctx, &request).await;

    // Decouple broadcast writes from stream yielding: the executor
    // sends each PRE-transform item into an UNBOUNDED channel (a send
    // never blocks, never drops), and a spawned writer task drains it
    // onto the producer socket. `run` owns the writer's lifecycle: the
    // stream it returns awaits the writer's completion after the
    // command finishes, so a program exit can never truncate in-flight
    // socket writes.
    let (tee_tx, tee_writer) = match feed {
        Some(TeeHandle { broadcast, id }) => {
            let (tx, mut rx) =
                tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
            let writer = tokio::spawn(async move {
                // Frame each pre-transform item as the bare
                // `ListenerResponse` `{id, value}` and fan it out on the
                // resident `/listen` broadcast (this framing was done by
                // the daemon socket's `handle_feed`; now it is in-process).
                while let Some(value) = rx.recv().await {
                    let frame =
                        serde_json::json!({ "id": id.clone(), "value": value }).to_string();
                    let _ = broadcast.send(frame);
                }
                // Channel closed → the run is complete. Broadcast exactly
                // one terminator so consumers can end this id's stream.
                if let Ok(frame) = serde_json::to_string(&ListenerEnd { id, end: true }) {
                    let _ = broadcast.send(frame);
                }
            });
            (Some(tx), Some(writer))
        }
        None => (None, None),
    };

    // Drive the request through the in-process executor, picking the
    // SDK root dispatch entry by whether the request carries an output
    // transform. The executor tees the PRE-transform items to the
    // broadcast and applies the transform / token / timeout adapters;
    // `execute_transform` additionally sets the transform on the leaf
    // request and yields the post-transform JSON, whereas `execute`
    // yields the typed root items.
    let transform = request.request_base().transform();
    let executor = crate::executor::DaemonCommandExecutor::new(ctx, tee_tx);
    match transform {
        Some(transform) => {
            let stream =
                objectiveai_sdk::cli::command::execute_transform(&executor, request, transform, None)
                    .await;
            drop(executor);
            match stream {
                Ok(stream) => Ok(RunStream::ExecuteTransform(await_tee_completion(
                    stream, tee_writer,
                ))),
                Err(e) => {
                    settle_tee(tee_writer).await;
                    Err(e)
                }
            }
        }
        None => {
            let stream =
                objectiveai_sdk::cli::command::execute(&executor, request, None).await;
            drop(executor);
            match stream {
                Ok(stream) => {
                    Ok(RunStream::Execute(await_tee_completion(stream, tee_writer)))
                }
                Err(e) => {
                    settle_tee(tee_writer).await;
                    Err(e)
                }
            }
        }
    }
    })
}

/// Whether this run should be teed into the daemon broadcast socket.
/// Everything is teed EXCEPT the resident daemon's own foreground process
/// (`daemon spawn --foreground`): its socket isn't bound and its lock
/// isn't published yet when the tee runs, so teeing it would
/// deadlock / fork-bomb daemon startup. Every real command — including the
/// `daemon spawn` launcher, `daemon kill`, and `kill_all` — is teed.
fn should_tee(request: &objectiveai_sdk::cli::command::Request) -> bool {
    use objectiveai_sdk::cli::command::{Request, daemon};
    !matches!(
        request,
        Request::Daemon(daemon::Request::Spawn(r))
            if r.dangerous_advanced.as_ref().and_then(|a| a.foreground) == Some(true)
    )
}

/// The producer's agent/plugin context object — exactly the fields the
/// `ListenerRequest<T>` wrapper carries. `None`s are omitted.
fn tee_context(config: &Config) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert(
        "agent_instance_hierarchy".to_string(),
        serde_json::Value::String(config.agent_instance_hierarchy.clone()),
    );
    for (key, value) in [
        ("agent_id", &config.agent_id),
        ("agent_full_id", &config.agent_full_id),
        ("agent_remote", &config.agent_remote),
        ("response_id", &config.response_id),
        ("response_ids", &config.response_ids),
    ] {
        if let Some(val) = value {
            map.insert(key.to_string(), serde_json::Value::String(val.clone()));
        }
    }
    serde_json::Value::Object(map)
}

/// In-process tee handle: the resident `/listen` broadcast sender plus
/// this run's frame id. The direct in-process replacement for the former
/// producer socket (`FeedWriter`).
struct TeeHandle {
    broadcast: tokio::sync::broadcast::Sender<String>,
    id: String,
}

/// Open an in-process tee onto the resident `/listen` broadcast: record
/// the daemon address and emit the opening `ListenerRequest` frame.
/// Best-effort: returns `None` (no teeing) for a non-teed request or when
/// this context isn't the resident daemon (no hubs → the same silent
/// fallback the failed socket connect gave).
async fn start_tee(ctx: &Context, request: &objectiveai_sdk::cli::command::Request) -> Option<TeeHandle> {
    if !should_tee(request) {
        return None;
    }
    // Idempotent: records the daemon's published `ws://` URL on the ctx
    // (used by `viewer spawn`). In-process this is a lock read — the
    // daemon is already up (we are it), so no spawn happens.
    if let Ok(url) = crate::command::daemon::spawn::spawn(ctx).await {
        ctx.set_daemon_address(url);
    }
    let broadcast = ctx.resident_hubs()?.broadcast.clone();
    let id = uuid::Uuid::new_v4().to_string();
    // Opening frame: the `ListenerRequest` shape — the producer context
    // map with `id` + `value` (the request) stamped in. (Was framed by
    // the daemon socket's `handle_feed`; now it is in-process.)
    let mut envelope = match tee_context(&ctx.config) {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    envelope.insert("id".to_string(), serde_json::Value::String(id.clone()));
    envelope.insert("value".to_string(), serde_json::to_value(request).ok()?);
    let _ = broadcast.send(serde_json::Value::Object(envelope).to_string());
    Some(TeeHandle { broadcast, id })
}

/// Wrap the command stream so that, after it completes, the run
/// AWAITS the broadcast writer task before ending — the consumer
/// (main.rs stdout drain, the daemon's `/execute` drain) cannot
/// finish until every queued socket write has landed, so process
/// exit never truncates the broadcast. The inner stream (which holds
/// the executor's tee sender) is dropped first, closing the channel
/// so the writer drains out and exits.
fn await_tee_completion<T>(
    stream: Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>,
    writer: Option<tokio::task::JoinHandle<()>>,
) -> Pin<Box<dyn Stream<Item = Result<T, Error>> + Send>>
where
    T: Send + 'static,
{
    let Some(writer) = writer else {
        return stream;
    };
    Box::pin(async_stream::stream! {
        use futures::StreamExt;
        let mut inner = stream;
        while let Some(item) = inner.next().await {
            yield item;
        }
        // Drop the inner stream first: it owns the tee sender, and the
        // writer only finishes once every sender is gone.
        drop(inner);
        let _ = writer.await;
    })
}

/// Early-error path: the run failed before producing a stream. Drop
/// nothing extra (the executor and its sender are already gone at the
/// call sites) — just wait for the writer to flush and exit.
async fn settle_tee(writer: Option<tokio::task::JoinHandle<()>>) {
    if let Some(writer) = writer {
        let _ = writer.await;
    }
}
