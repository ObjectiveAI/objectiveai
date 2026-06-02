//! Plugins management + external plugin dispatcher.
//!
//! Two responsibilities live here:
//!
//! 1. The built-in `plugins` subcommand tree ([`Commands`]):
//!    - `plugins list` — enumerate every manifest in
//!      `~/.objectiveai/plugins/` via
//!      [`crate::filesystem::Client::list_plugins`].
//!    - `plugins <name> <args…>` — dispatch to plugin `<name>` with
//!      `<args…>` forwarded verbatim (the [`Commands::Run`] external
//!      subcommand). Identical behaviour to the top-level catch-all
//!      below; this is just the explicit, namespaced form.
//! 2. The top-level catch-all dispatcher ([`dispatch_external`]) invoked
//!    by clap's outer `external_subcommand` for any unknown top-level
//!    subcommand: `objectiveai <name> <args…>`.
//!
//! Both routes land in [`dispatch_external`], so the runtime behaviour
//! is identical — only the parse path differs. We resolve `<name>`
//! against `~/.objectiveai/plugins/`, spawn the binary with `<args…>`,
//! and consume its stdout as a JSONL stream of [`objectiveai_sdk::cli::plugins::Output`].
//! Per-line dispatch:
//!
//! - `Error` → forward via [`Output::Error`]
//! - `Notification(Value)` → forward via [`Output::Notification`]
//! - `Command { id, command }` → tokenize the string and spawn a
//!   recursive `cli::run` call. The spawned command's emissions
//!   ALWAYS flow back into the plugin's stdin **as they happen**,
//!   one envelope line per emit, followed by one terminal
//!   `command_complete` notification once the run finishes. Every
//!   line is wrapped in a `PluginCommandResponse { id, value }`
//!   envelope so the plugin parses a uniform shape for every
//!   command. When the plugin minted an `id`, the envelope's `id`
//!   field echoes it (`{"id":"<id>","value":{…}}`) so the plugin
//!   can demultiplex concurrent in-flight commands; when `id` is
//!   absent, the field is dropped on the wire (`{"value":{…}}`)
//!   and the plugin gets a single in-order stream delimited by
//!   `command_complete` markers. Nothing from a spawned command
//!   ever lands on the host's own stdout. Either way the wire is
//!   line-serialized on the shared `Arc<Mutex<ChildStdin>>`.
//! - parse failure → re-emit the raw line as a string-valued
//!   notification, so it still appears in the host's JSONL stream
//!
//! Plugin stdin is opened with [`Stdio::piped`] so the host can write
//! responses; plugins that don't read it just block their own input
//! pipe (no-op). Plugin stderr is forwarded raw to this process's own
//! stderr (same pattern as `api::detach`). The plugin's exit code
//! becomes this function's `Err(PluginExit)` — which `run()` then
//! emits as a fatal error and converts to exit code 1.

use std::sync::Arc;

use clap::Subcommand;
use objectiveai_sdk::cli::output::{
    CommandComplete, Handle, HandleDestination, Installed, Notification, Output, Plugin, Plugins,
    TypedNotificationValue,
};
use objectiveai_sdk::cli::plugins::{Output as PluginOutput, TypedOutput as TypedPluginOutput};
use tokio::io::AsyncBufReadExt;
use tokio::process::ChildStdin;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

mod install;

#[derive(Subcommand)]
pub enum Commands {
    /// Get a single plugin's manifest by name. Emits the manifest as
    /// `{"plugin": <manifest>}` when found, or `{"plugin": null}` when
    /// the manifest file is missing / unreadable / malformed (same
    /// silent-skip policy as `list`).
    Get {
        /// Plugin name (filename stem of the manifest in
        /// `~/.objectiveai/plugins/`).
        name: String,
    },
    /// Install a plugin from a GitHub repository. Fetches
    /// `objectiveai.json` from the repo root at `--commit-sha`
    /// (or the default branch if omitted), checks the (owner,
    /// repository, commit_sha_or_HEAD, manifest.version) quadruple
    /// against the install whitelist, then downloads the matching
    /// release asset for this platform and writes it to
    /// `~/.objectiveai/plugins/<repository>/plugin` (with `.exe` on
    /// Windows). Emits `{"installed": false}` when this platform
    /// isn't in the manifest's `binaries` map. Errors with
    /// `PluginNotWhitelisted` when the quadruple matches no
    /// whitelist entry and `--allow-untrusted` was not passed.
    ///
    /// Now subcommand-bearing: `plugins install github …` for the
    /// remote-install flow, `plugins install filesystem` for
    /// instructions on hand-authoring a local plugin. See
    /// [`install::Commands`].
    Install {
        #[command(subcommand)]
        command: install::Commands,
    },
    /// List installed plugins (every `.json` manifest in
    /// `~/.objectiveai/plugins/`). Sorted by manifest mtime, most
    /// recent first. Supports `--offset` / `--limit` for pagination,
    /// matching `agents completions logs list` and siblings.
    List {
        #[arg(long, default_value_t = 0)]
        offset: usize,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    /// Run a plugin from `~/.objectiveai/plugins/`. First element is
    /// the plugin name; the rest are forwarded as the plugin's argv
    /// verbatim. The shell handles tokenization — quoted args stay
    /// grouped, no flag parsing happens here. Identical dispatch to
    /// the top-level catch-all `objectiveai <name> <args…>`, just
    /// namespaced under `plugins`.
    #[command(external_subcommand)]
    Run(Vec<String>),
}

impl Commands {
    pub async fn handle(
        self,
        cli_config: &crate::Config,
        handle: &Handle,
    ) -> Result<(), crate::error::Error> {
        match self {
            Commands::Get { name } => get(cli_config, handle, &name).await,
            Commands::Install { command } => command.handle(cli_config, handle).await,
            Commands::List { offset, limit } => list(cli_config, handle, offset, limit).await,
            Commands::Run(args) => dispatch_external(args, cli_config, handle).await,
        }
    }
}

pub(super) async fn install(
    cli_config: &crate::Config,
    handle: &Handle,
    owner: &str,
    repository: &str,
    commit_sha: Option<&str>,
    allow_untrusted: bool,
    upgrade: bool,
) -> Result<(), crate::error::Error> {
    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );

    // Step 1: fetch the manifest so we can extract its version for
    // the whitelist check.
    let manifest = fs_client
        .fetch_plugin_manifest(owner, repository, commit_sha, None)
        .await?;

    // Step 2: whitelist check.
    let effective_sha = commit_sha.unwrap_or("HEAD");
    let whitelist = crate::filesystem::plugins::default_whitelist();
    let allowed = crate::filesystem::plugins::check_plugin_whitelist(
        owner,
        repository,
        effective_sha,
        &manifest.version,
        &whitelist,
    )
    .map_err(crate::error::Error::WhitelistRegex)?;

    if !allowed {
        if !allow_untrusted {
            return Err(crate::error::Error::PluginNotWhitelisted {
                owner: owner.to_string(),
                repository: repository.to_string(),
                commit_sha: effective_sha.to_string(),
                version: manifest.version.clone(),
            });
        }
        emit_untrusted_warning(handle, owner, repository, effective_sha, &manifest.version).await;
    }

    // Step 3: install from the already-fetched manifest. The source
    // URL persisted alongside the binary is the raw GitHub URL the
    // manifest came from.
    let source =
        crate::filesystem::plugins::raw_manifest_url(owner, repository, commit_sha);
    let installed = fs_client
        .install_plugin_from_manifest(owner, repository, &manifest, &source, None, upgrade)
        .await?;
    Output::Notification(Notification {
        value: (Installed { installed }).into(),
    })
    .emit(handle)
    .await;
    Ok(())
}

async fn emit_untrusted_warning(
    handle: &Handle,
    owner: &str,
    repository: &str,
    commit_sha: &str,
    version: &str,
) {
    use objectiveai_sdk::cli::output::{Error as OutputError, ErrorType, Level};
    let message = format!(
        "installing untrusted plugin {owner}/{repository} (commit: {commit_sha}, version: {version}); \
         this plugin is not in the whitelist and is being installed because --allow-untrusted was passed"
    );
    Output::Error(OutputError {
        r#type: ErrorType::Error,
        level: Level::Warn,
        fatal: false,
        message: message.into(),
        agent_instance_hierarchy: None,
    })
    .emit(handle)
    .await;
}

async fn get(
    cli_config: &crate::Config,
    handle: &Handle,
    name: &str,
) -> Result<(), crate::error::Error> {
    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let plugin = fs_client.get_plugin(name).await;
    Output::Notification(Notification {
        value: (Plugin { plugin }).into(),
    })
    .emit(handle)
    .await;
    Ok(())
}

async fn list(
    cli_config: &crate::Config,
    handle: &Handle,
    offset: usize,
    limit: usize,
) -> Result<(), crate::error::Error> {
    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let plugins = fs_client.list_plugins(offset, limit).await;
    Output::Notification(Notification {
        value: (Plugins { plugins }).into(),
    })
    .emit(handle)
    .await;
    Ok(())
}

pub async fn dispatch_external(
    args: Vec<String>,
    cli_config: &crate::Config,
    handle: &Handle,
) -> Result<(), crate::error::Error> {
    let mut iter = args.into_iter();
    let name_str = iter
        .next()
        .ok_or(crate::error::Error::MissingArgs("plugin name"))?;
    let rest: Vec<String> = iter.collect();

    let fs_client = crate::filesystem::Client::new(
        cli_config.config_base_dir.as_deref(),
        cli_config.commit_author_name.as_deref(),
        cli_config.commit_author_email.as_deref(),
    );
    let exe = match fs_client.resolve_plugin(&name_str).await {
        Some(p) => p,
        None => return Err(crate::error::Error::PluginNotFound(name_str)),
    };

    let mut cmd = tokio::process::Command::new(&exe);
    cmd.args(&rest)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    // Forward the full cli Config (agent lineage, MCP session, auth
    // tokens, config_base_dir, the "we're under objectiveai-mcp"
    // marker, …) to the plugin subprocess via the same env-var
    // names `EnvConfigBuilder` reads.
    crate::spawn::apply_config_env(&mut cmd, cli_config);

    let mut child = cmd.spawn().map_err(crate::error::Error::PluginSpawn)?;

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let plugin_stdin: Arc<Mutex<ChildStdin>> = Arc::new(Mutex::new(child.stdin.take().unwrap()));

    let stderr_task = tokio::spawn(forward_stderr(stderr));

    // (request_id, run JoinHandle). When `request_id` is Some, the
    // command's emissions are flowing back into the plugin's stdin
    // and the dispatcher emits a terminal `CommandComplete` line
    // (stamped with the same id) after the run resolves. When None
    // it's legacy fire-and-forget — no terminal marker, output went
    // to `handle`.
    let mut command_tasks: Vec<(Option<String>, JoinHandle<i32>)> = Vec::new();
    let mut reader = tokio::io::BufReader::new(stdout);
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(crate::error::Error::PluginRead)?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        match serde_json::from_str::<PluginOutput>(trimmed) {
            Ok(PluginOutput::Typed(TypedPluginOutput::Error(e))) => {
                Output::Error(e).emit(handle).await;
            }
            Ok(PluginOutput::Typed(TypedPluginOutput::Mcp(mcp))) => {
                // Forward to the host's notification stream so
                // subscribers see it on the standard channel — the
                // wire variant is direct, but the host's emit
                // surface keeps the `Mcp` value typed.
                Output::Notification(Notification { value: mcp.into() })
                    .emit(handle)
                    .await;
            }
            Ok(PluginOutput::Notification(value)) => {
                Output::Notification(Notification {
                    value: objectiveai_sdk::cli::output::NotificationValue::Typed(
                        TypedNotificationValue::PluginNotification { value },
                    ),
                })
                .emit(handle)
                .await;
            }
            Ok(PluginOutput::Typed(TypedPluginOutput::Command { id, command })) => {
                // Per-command handle: writes back to the plugin's
                // stdin in every case, wrapping each emitted line in
                // a `PluginCommandResponse { id, value }` envelope.
                // When the plugin minted an `id`, the envelope's
                // `id` field echoes it so the plugin can demultiplex
                // concurrent commands; when there's no id, the
                // field is dropped on the wire and the plugin sees a
                // single in-order stream delimited by
                // `command_complete` markers.
                let task_handle = Handle::from(HandleDestination::PluginCommandStdin {
                    stdin: plugin_stdin.clone(),
                    id: id.clone(),
                });
                command_tasks.push((id, spawn_command(command, cli_config, task_handle)));
            }
            Err(_) => {
                let value = serde_json::Value::String(trimmed.to_string());
                Output::Notification(Notification {
                    value: objectiveai_sdk::cli::output::NotificationValue::Typed(
                        TypedNotificationValue::PluginNotification { value },
                    ),
                })
                .emit(handle)
                .await;
            }
        }
    }

    // Drain any in-flight Command runs the plugin queued before
    // exiting. Each task gets a terminal `CommandComplete` on the
    // plugin's stdin so the plugin always knows the run finished,
    // even when it didn't mint a correlation id. Hold the
    // `plugin_stdin` Arc until every marker has been written;
    // dropping it earlier would close the plugin's stdin and lose
    // the final lines.
    for (id, t) in command_tasks {
        let exit_code = t.await.unwrap_or(-1);
        let completion_handle = Handle::from(HandleDestination::PluginCommandStdin {
            stdin: plugin_stdin.clone(),
            id,
        });
        Output::Notification(Notification {
            value: CommandComplete { exit_code }.into(),
        })
        .emit(&completion_handle)
        .await;
    }
    // Now safe to drop the dispatcher's reference; once the plugin
    // closes its stdin handle the kernel pipe closes and a polite
    // plugin sees EOF on its stdin read.
    drop(plugin_stdin);
    let _ = stderr_task.await;

    let status = child
        .wait()
        .await
        .map_err(crate::error::Error::PluginRead)?;
    if status.success() {
        Ok(())
    } else {
        Err(crate::error::Error::PluginExit(status.code().unwrap_or(1)))
    }
}

/// Tokenize the plugin's `command` string and spawn `cli::run` on it.
/// Uses whitespace splitting — quoted args aren't supported (upgrade
/// to `shlex` if a real plugin needs them).
///
/// `handle` carries the destination + correlation id for THIS command's
/// emissions. For id-bearing dispatches it's a stdin-routed handle
/// stamped with the plugin's request_id; for legacy fire-and-forget
/// dispatches it's the dispatcher's own handle.
fn spawn_command(command: String, cli_config: &crate::Config, handle: Handle) -> JoinHandle<i32> {
    let tokens: Vec<String> = command.split_whitespace().map(String::from).collect();
    // `run()` expects argv[0] to be the binary name (clap ignores its
    // content). Prepend a placeholder.
    let mut argv: Vec<String> = Vec::with_capacity(tokens.len() + 1);
    argv.push(String::from("objectiveai"));
    argv.extend(tokens);

    let cfg = cli_config.clone();
    tokio::spawn(async move { crate::run::run(argv, &cfg, handle).await })
}

async fn forward_stderr(mut stderr: tokio::process::ChildStderr) {
    use tokio::io::AsyncReadExt;
    let mut buf = [0u8; 4096];
    loop {
        match stderr.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                use std::io::Write;
                let _ = std::io::stderr().write_all(&buf[..n]);
            }
        }
    }
}
