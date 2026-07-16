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
///    (`CREATE_NO_WINDOW | DETACHED_PROCESS` — except the `viewer`
///    spawn, whose `configure` overrides to `DETACHED_PROCESS` only:
///    the viewer is a windowed app); `kill_on_drop` stays
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

/// Spawn one of the daemon's persistent servers (`db` / `api` / `mcp`
/// / `viewer` / `laboratories`) as an OS-LEASHED child and wait for
/// its stdout readiness handshake
/// ([`objectiveai_sdk::process::ServerReady`]). Returns the announced
/// address (`None` for listener-less servers — viewer, laboratory
/// host).
///
/// This replaced the lockfile-readiness spawn: the daemon is the sole
/// spawner and OWNS the server's lifetime — the leash
/// ([`objectiveai_sdk::subprocess_reaper`]) makes the OS kill the
/// child when the daemon dies by ANY means, and the held
/// [`tokio::process::Child`] on the [`Context`] keeps it alive
/// meanwhile. Singleton-per-key is the `resident_children` map plus
/// the per-key spawn gate (no cross-process lock: nothing else is
/// allowed to spawn these).
///
/// Flow, under the key's spawn gate:
/// 1. A LIVE cached child ⇒ return its cached address (idempotent).
///    A dead one is dropped and respawned.
/// 2. Spawn leashed: null stdin, piped stdout/stderr (the pipes ride
///    [`crate::child_io::spawn_pipe_reader`] — reader tasks own them,
///    so there is no partial-line or cancel-safety hazard), Windows
///    `CREATE_NO_WINDOW` only — leashed children stay
///    console-attached (the job object is the death leash; DETACHED
///    would exempt them from Ctrl+C for no benefit, and they die with
///    the daemon regardless).
/// 3. Read stdout lines until one parses as the ready line, racing
///    the child's exit — an exit first drains the pipes briefly and
///    errors with the captured output.
/// 4. Park the child (+ address) on the `Context` and hand the SAME
///    pipe receiver to a persistent drain task for the child's life,
///    so later server writes never block the pipe or EPIPE-kill
///    anyone.
pub async fn spawn_leashed_until_ready(
    ctx: &crate::context::Context,
    key: &str,
    exe: &Path,
    configure: impl FnOnce(&mut Command),
) -> Result<Option<String>, crate::error::Error> {
    let gate = ctx.spawn_gate(key);
    let _guard = gate.lock().await;

    if let Some(address) = ctx.resident_child_address(key) {
        return Ok(address);
    }

    let name = exe
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string());

    let mut cmd = Command::new(exe);
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    configure(&mut cmd);

    let mut child = objectiveai_sdk::subprocess_reaper::spawn(&mut cmd)
        .map_err(|e| crate::error::Error::Spawn(name.clone(), e))?;
    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");
    let mut events = crate::child_io::spawn_pipe_reader(stdout, stderr);

    // Collected for the exited-before-ready error report only.
    let mut seen_stdout: Vec<String> = Vec::new();
    let mut seen_stderr: Vec<String> = Vec::new();
    let address = loop {
        tokio::select! {
            event = events.recv() => match event {
                Some(crate::child_io::PipeEvent::Stdout(line)) => {
                    if let Some(ready) = objectiveai_sdk::process::parse_ready(&line) {
                        break ready.address;
                    }
                    seen_stdout.push(line);
                }
                Some(crate::child_io::PipeEvent::Stderr(line)) => {
                    seen_stderr.push(line);
                }
                // EOFs / read errors without a ready line: keep
                // waiting — the child-exit arm is the terminal signal.
                Some(_) => {}
                None => {}
            },
            status = child.wait() => {
                // Exited before announcing readiness. Give the reader
                // tasks a moment to flush what the child wrote, then
                // report it.
                let deadline = tokio::time::Instant::now()
                    + std::time::Duration::from_secs(2);
                loop {
                    match tokio::time::timeout_at(deadline, events.recv()).await {
                        Ok(Some(crate::child_io::PipeEvent::Stdout(line))) => {
                            seen_stdout.push(line);
                        }
                        Ok(Some(crate::child_io::PipeEvent::Stderr(line))) => {
                            seen_stderr.push(line);
                        }
                        Ok(Some(_)) => {}
                        Ok(None) | Err(_) => break,
                    }
                }
                return Err(crate::error::Error::SpawnExitedBeforePublishing {
                    name,
                    status: status.map_err(|e| {
                        crate::error::Error::Spawn(name_of(exe), e)
                    })?,
                    stdout: seen_stdout.join("\n"),
                    stderr: seen_stderr.join("\n"),
                });
            }
        }
    };

    // Persistent drain: the child's std handles stay live for its
    // whole life — later writes (db's supervisor notices, api's cost
    // lines) are consumed and DISCARDED instead of blocking the pipe
    // or EPIPE-killing the server. Anything observable goes through
    // the server's published channel or the DB, per the standing
    // convention.
    tokio::spawn(async move { while events.recv().await.is_some() {} });

    ctx.hold_resident_child(key, child, address.clone());
    Ok(address)
}

/// Basename-or-path display name for spawn errors.
fn name_of(exe: &Path) -> String {
    exe.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string())
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
    // NOTE: the daemon's own bind config (bare `ADDRESS`/`PORT`/`SECRET`)
    // is deliberately NOT projected here — it is server-specific and would
    // pollute a plugin/tool child's generic `$PORT`/`$SECRET`. Only the
    // foreground-daemon spawn stamps them, via `apply_daemon_env` below.
}

/// Stamp the daemon's own bind config as the BARE
/// `ADDRESS`/`PORT`/`SECRET`/`SIGNATURE` env the daemon reads (see
/// `run::EnvConfigBuilder`). Used ONLY when spawning the resident
/// foreground daemon — never for plugins/tools, which must not inherit
/// these generic-named vars. Address/port always carry resolved
/// defaults; the secret and pre-derived client signature are set when
/// present and cleared otherwise so the child can't inherit stale ones.
pub fn apply_daemon_env(cmd: &mut Command, cfg: &crate::Config) {
    cmd.env("ADDRESS", &cfg.daemon_address);
    cmd.env("PORT", cfg.daemon_port.to_string());
    match cfg.daemon_secret.as_deref() {
        Some(v) => {
            cmd.env("SECRET", v);
        }
        None => {
            cmd.env_remove("SECRET");
        }
    }
    match cfg.daemon_signature.as_deref() {
        Some(v) => {
            cmd.env("SIGNATURE", v);
        }
        None => {
            cmd.env_remove("SIGNATURE");
        }
    }
}
