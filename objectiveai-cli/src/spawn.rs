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
/// 2. Otherwise spawn `exe` with a FRESH environment (`env_clear`): the
///    child inherits nothing from the cli except what `configure`
///    explicitly sets. Null stdio; detached from the console on Windows
///    (`CREATE_NO_WINDOW | DETACHED_PROCESS`); `kill_on_drop` stays
///    false everywhere so the child outlives the cli (Unix re-parents
///    it to init when the cli exits).
/// 3. Subscribe to the lock ([`objectiveai_sdk::lockfile::wait_read`]),
///    racing the child's exit. Lock published → the server is up and
///    its URL is returned. Child exited first → the server failed to
///    start (or lost a concurrent claim race) → error; without this arm
///    the subscribing read would wait forever on a dead child.
pub async fn spawn_until_lock_published(
    exe: &Path,
    lock_dir: &Path,
    key: &str,
    configure: impl FnOnce(&mut Command),
) -> Result<String, crate::error::Error> {
    let lock_err = |e: std::io::Error| crate::error::Error::Lockfile {
        key: key.to_string(),
        source: e,
    };

    if let Some(listening) = objectiveai_sdk::lockfile::try_read(lock_dir, key)
        .await
        .map_err(lock_err)?
    {
        return Ok(listening);
    }

    let name = exe
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string());

    let mut cmd = Command::new(exe);
    cmd.env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW (0x08000000) | DETACHED_PROCESS (0x00000008)
        // — keep the spawned binary off the parent console and let it
        // outlive the cli.
        cmd.creation_flags(0x0800_0008);
        // SYSTEMROOT survives the env_clear: winsock (WSAStartup) — and
        // therefore every socket the servers bind — fails without it,
        // as does the postmaster objectiveai-db launches. It's machine
        // identity, not inherited cli config.
        if let Some(system_root) = std::env::var_os("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", system_root);
        }
    }
    configure(&mut cmd);

    let mut child = cmd
        .spawn()
        .map_err(|e| crate::error::Error::Spawn(name.clone(), e))?;

    let listening = tokio::select! {
        read = objectiveai_sdk::lockfile::wait_read(lock_dir, key) => read.map_err(lock_err)?,
        _ = child.wait() => {
            return Err(crate::error::Error::SpawnExitedBeforePublishing { name });
        }
    };

    // tokio's Child drops without killing (kill_on_drop is false by
    // default), so the spawned binary is detached: on Unix the kernel
    // re-parents it to init when the cli exits; on Windows the parent's
    // handle is released and the spawned binary continues.
    drop(child);

    Ok(listening)
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
    if let Some(v) = cfg.github_authorization.as_deref() {
        cmd.env("GITHUB_AUTHORIZATION", v);
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
    // Plugin coordinate — set when a nested command is launched on
    // behalf of a plugin; removed otherwise so a child can't inherit a
    // stale plugin identity from the parent's startup environment.
    match cfg.plugin_owner.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_OWNER", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_OWNER");
        }
    }
    match cfg.plugin_repository.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_REPOSITORY", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_REPOSITORY");
        }
    }
    match cfg.plugin_version.as_deref() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_VERSION", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_VERSION");
        }
    }
}
