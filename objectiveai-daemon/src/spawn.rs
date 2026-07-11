//! Shared process-lifecycle primitives for the `{api,viewer,mcp,db}
//! spawn|kill` cli subcommands.

use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};
use tokio::process::Command;

/// Send SIGTERM (Unix) / TerminateProcess (Windows) to one specific
/// pid. Returns 1 if a live process with that pid existed and was
/// targeted, 0 otherwise. Used by `db kill`, where the postmaster's
/// pid comes from `postmaster.pid` and a name match would hit
/// unrelated postgres servers.
pub fn kill_pid(pid: u32) -> usize {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    match sys.process(sysinfo::Pid::from_u32(pid)) {
        Some(process) => {
            let _ = process
                .kill_with(Signal::Term)
                .or_else(|| Some(process.kill()));
            1
        }
        None => 0,
    }
}

/// Absolutize a relative exec *path* against `cwd`; keep bare names'
/// PATH-lookup semantics. Shared by `tools run` and `plugins run`,
/// whose manifest exec paths are relative to their version / `cli`
/// folder (e.g. `./count-tool.exe`) — but on Windows `CreateProcess`
/// resolves a relative program against the PARENT's cwd, not the
/// child's `current_dir` (rust-lang/rust#37868), so the spawn would
/// miss the binary entirely without this.
///
/// Path-vs-name is decided by `Path::components()`, which encodes
/// the platform split for us:
///   - Windows: `/` and `\` are both separators (and both illegal
///     in file names), so either marks a path — 2+ components.
///   - Unix: only `/` separates; `\` is a legal filename byte, so
///     a program literally named `my\tool` stays a bare name —
///     1 component — and still resolves via PATH.
pub fn resolve_program(program: String, cwd: &Path) -> std::ffi::OsString {
    let path = Path::new(&program);
    if path.components().count() > 1 && path.is_relative() {
        cwd.join(path).into_os_string()
    } else {
        program.into()
    }
}

/// Lock-based background spawn shared by the four `* spawn` commands.
///
/// A server's readiness signal is its lockfile: once its listener is
/// bound, each server claims `(lock_dir, key)` via
/// [`objectiveai_sdk::lockfile`] and publishes its client-connect URL
/// as the lock contents. The flow:
///
/// 1. [`objectiveai_sdk::lockfile::try_read`] — if the lock is already
///    held by a live owner, the server is already up; return its
///    published URL without spawning anything.
/// 2. Otherwise spawn `exe`. The child INHERITS the cli's environment
///    — the dev/test `bin` entries are cargo-run shims, and a build
///    needs the full machine environment (PATH, the MSVC toolchain
///    discovery vars, cargo/rustup homes). Config isolation is the
///    spawn commands' job instead: each `configure` explicitly
///    `env_remove`s every env key its server binary's config struct
///    reads that it doesn't deliberately set, so the spawning shell's
///    configuration never leaks into a server other processes will
///    share. Null stdin; stdout/stderr are piped and drained so a
///    child that dies before publishing reports its own output in
///    the error; detached from the console on Windows
///    (`CREATE_NO_WINDOW | DETACHED_PROCESS`); `kill_on_drop` stays
///    false everywhere so the child outlives the cli (Unix re-parents
///    it to init when the cli exits).
/// 3. Subscribe to the lock ([`objectiveai_sdk::lockfile::wait_read`]),
///    racing the child's exit. Lock published → the server is up and
///    its URL is returned. Child exited first → one last `try_read`:
///    the child may have died because it lost the claim race to a
///    concurrently spawned server, and a held lock now means one won
///    in reality — return its URL. Only a dead child AND a free lock
///    is a failure. (Without the child arm the subscribing read would
///    wait forever on a dead child.)
pub async fn spawn_until_lock_published(
    exe: &Path,
    lock_dir: &Path,
    key: &str,
    configure: impl FnOnce(&mut Command),
) -> Result<String, crate::error::Error> {
    // The discipline itself lives in the SDK (shared with the viewer
    // shell's laboratory commands); this wrapper only maps errors into
    // the cli's variants.
    objectiveai_sdk::lockfile::spawn_until_published(exe, lock_dir, key, configure)
        .await
        .map_err(|e| match e {
            objectiveai_sdk::lockfile::SpawnPublishError::Lock(source) => {
                crate::error::Error::Lockfile { key: key.to_string(), source }
            }
            objectiveai_sdk::lockfile::SpawnPublishError::Spawn(source) => {
                crate::error::Error::Spawn(
                    exe.file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| exe.display().to_string()),
                    source,
                )
            }
            objectiveai_sdk::lockfile::SpawnPublishError::ExitedBeforePublishing {
                name,
                status,
                stdout,
                stderr,
            } => crate::error::Error::SpawnExitedBeforePublishing {
                name,
                status,
                stdout,
                stderr,
            },
        })
}

/// Stamp every field of the cli's [`crate::Config`] onto `cmd`'s env
/// using the same env-var names the [`crate::run::EnvConfigBuilder`]
/// reads on the receiving side. So a child cli (or any subprocess
/// that uses the same `Envconfig`-based loader) round-trips its
/// parent's config byte-identically.
///
/// `Option`-typed fields are skipped on `None`, EXCEPT the six
/// per-request transient identity keys (`OBJECTIVEAI_AGENT_ID`,
/// `_FULL_ID`, `_REMOTE`, `_RESPONSE_ID`, `_RESPONSE_IDS`, and
/// `MCP_SESSION_ID`), which are `env_remove`'d on `None` so the
/// child cannot inherit a stale identity from the parent's startup
/// environment. Boolean fields are stamped only when `true`.
pub fn apply_config_env(cmd: &mut Command, cfg: &crate::Config) {
    if cfg.config_set_forbidden {
        cmd.env("CONFIG_SET_FORBIDDEN", "true");
    }
    if let Some(v) = cfg.objectiveai_dir.as_deref() {
        cmd.env("OBJECTIVEAI_DIR", v);
    }
    if let Some(v) = cfg.objectiveai_state.as_deref() {
        cmd.env("OBJECTIVEAI_STATE", v);
    }
    if let Some(v) = cfg.commit_author_name.as_deref() {
        cmd.env("COMMIT_AUTHOR_NAME", v);
    }
    if let Some(v) = cfg.commit_author_email.as_deref() {
        cmd.env("COMMIT_AUTHOR_EMAIL", v);
    }
    cmd.env("OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY", &cfg.agent_instance_hierarchy);
    match cfg.agent_id.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_ID");
        }
    }
    match cfg.agent_full_id.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_FULL_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_FULL_ID");
        }
    }
    match cfg.agent_remote.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_REMOTE", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_REMOTE");
        }
    }
    match cfg.response_id.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_RESPONSE_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_RESPONSE_ID");
        }
    }
    match cfg.response_ids.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_RESPONSE_IDS", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_RESPONSE_IDS");
        }
    }
    match cfg.mcp_session_id.as_deref() {
        Some(v) => {
            cmd.env(objectiveai_sdk::mcp::MCP_SESSION_ID_ENV, v);
        }
        None => {
            cmd.env_remove(objectiveai_sdk::mcp::MCP_SESSION_ID_ENV);
        }
    }
    // Resident-daemon broadcast WebSocket bind address/port. Always
    // projected (both carry resolved defaults), so the re-exec'd
    // foreground daemon inherits the launcher's configured listener.
    cmd.env("DAEMON_ADDRESS", &cfg.daemon_address);
    cmd.env("DAEMON_PORT", cfg.daemon_port.to_string());
    // Optional daemon WebSocket auth secret — set when present, cleared
    // otherwise so a child can't inherit a stale secret from the parent's
    // startup environment.
    match cfg.daemon_secret.as_deref() {
        Some(v) => {
            cmd.env("DAEMON_SECRET", v);
        }
        None => {
            cmd.env_remove("DAEMON_SECRET");
        }
    }
}
