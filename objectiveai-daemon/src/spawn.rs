//! Shared process-lifecycle primitives for the daemon's spawn
//! commands.
//!
//! Two spawn disciplines live here:
//!
//! - [`spawn_leashed_until_ready`] — the daemon's five persistent
//!   servers (`db`/`api`/`mcp`/`viewer`/`laboratories`): OS-leashed
//!   children that die with the daemon and announce readiness over a
//!   stdout JSON handshake.
//! - [`spawn_until_lock_published`] — peer plugins-daemons only
//!   (`daemon spawn`): deliberately DETACHED, because a daemon for
//!   another state must outlive whichever daemon happened to spawn
//!   it, and discovered through the `plugins-daemon` lockfile — the
//!   one rendezvous that must work across unrelated processes (cli
//!   bootstrap).

use std::path::Path;

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};
use tokio::process::Command;

/// Send SIGTERM (Unix) / TerminateProcess (Windows) to one specific
/// pid. Returns 1 if a live process with that pid existed and was
/// targeted, 0 otherwise. Kills strictly by pid — a name match would
/// hit unrelated processes (e.g. other postgres servers). Used by the
/// legacy lock-owner sweep in `command::kill_helpers`.
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

/// Lock-based DETACHED spawn — used only by `daemon spawn` for peer
/// plugins-daemons (the persistent servers use
/// [`spawn_leashed_until_ready`] instead).
///
/// The daemon's readiness signal is its lockfile: once its listener is
/// bound, it claims `(lock_dir, key)` via
/// [`objectiveai_sdk::lockfile`] and publishes its client-connect URL
/// as the lock contents. The flow:
///
/// 1. [`objectiveai_sdk::lockfile::try_read`] — if the lock is already
///    held by a live owner, the daemon is already up; return its
///    published URL without spawning anything.
/// 2. Otherwise spawn `exe`. The child INHERITS the spawner's
///    environment — the dev/test `bin` entries are cargo-run shims,
///    and a build needs the full machine environment (PATH, the MSVC
///    toolchain discovery vars, cargo/rustup homes). Config isolation
///    is the spawn command's job instead: `configure` explicitly
///    `env_remove`s every env key the child's config struct reads
///    that it doesn't deliberately set, so the spawning shell's
///    configuration never leaks into a daemon other processes will
///    share. Null stdin; stdout/stderr are piped and drained so a
///    child that dies before publishing reports its own output in
///    the error; detached from the console on Windows
///    (`CREATE_NO_WINDOW | DETACHED_PROCESS`); `kill_on_drop` stays
///    false so the child outlives its spawner (Unix re-parents it to
///    init when the spawner exits).
/// 3. Subscribe to the lock ([`objectiveai_sdk::lockfile::wait_read`]),
///    racing the child's exit. Lock published → the daemon is up and
///    its URL is returned. Child exited first → one last `try_read`:
///    the child may have died because it lost the claim race to a
///    concurrently spawned daemon, and a held lock now means one won
///    in reality — return its URL. Only a dead child AND a free lock
///    is a failure. (Without the child arm the subscribing read would
///    wait forever on a dead child.)
pub async fn spawn_until_lock_published(
    exe: &Path,
    lock_dir: &Path,
    key: &str,
    configure: impl FnOnce(&mut Command),
) -> Result<String, crate::error::Error> {
    // The discipline itself lives in the SDK (shared with the cli's
    // ensure-daemon bootstrap); this wrapper only maps errors into
    // the daemon's variants.
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
/// / `viewer`) as an OS-LEASHED child and wait for its stdout
/// readiness handshake ([`objectiveai_sdk::process::ServerReady`]).
/// Returns the announced address (`None` for listener-less servers —
/// the viewer). The laboratory host uses
/// [`spawn_leashed_until_ready_with_stdio`] instead.
///
/// This replaced the lockfile-readiness spawn: the daemon is the sole
/// spawner and OWNS the server's lifetime — the leash
/// ([`objectiveai_sdk::subprocess_reaper`]) makes the OS kill the
/// child when the daemon dies by ANY means, and the held
/// [`tokio::process::Child`] on the [`crate::context::GlobalContext`] keeps it alive
/// meanwhile. Singleton-per-key is the `resident_children` map plus
/// the per-key spawn gate (no cross-process lock: nothing else is
/// allowed to spawn these).
///
/// Flow, under the key's spawn gate:
/// 1. A LIVE cached child ⇒ return its cached address (idempotent).
///    A dead one is dropped and respawned.
/// 2. Spawn leashed: null stdin (piped in the stdio variant), piped
///    stdout/stderr (the pipes ride
///    [`crate::child_io::spawn_pipe_reader`] — reader tasks own them,
///    so there is no partial-line or cancel-safety hazard), Windows
///    `CREATE_NO_WINDOW` only — leashed children stay
///    console-attached (the job object is the death leash; DETACHED
///    would exempt them from Ctrl+C for no benefit, and they die with
///    the daemon regardless).
/// 3. Read stdout lines until one parses as the ready line, racing
///    the child's exit — an exit first drains the pipes briefly and
///    errors with the captured output.
/// 4. Park the child (+ address) on the `GlobalContext` and hand the SAME
///    pipe receiver to a persistent drain task (the ack router, in
///    the stdio variant) for the child's life, so later server writes
///    never block the pipe or EPIPE-kill anyone.
pub async fn spawn_leashed_until_ready(
    global: &crate::context::GlobalContext,
    key: &str,
    exe: &Path,
    configure: impl FnOnce(&mut Command),
) -> Result<Option<String>, crate::error::Error> {
    spawn_leashed_inner(global, key, exe, configure, false)
        .await
        .map(|(address, _freshly_spawned)| address)
}

/// [`spawn_leashed_until_ready`] for the laboratory host: stdin is
/// PIPED (the host's stdin dial-list channel) and, after the ready
/// line, the pipe receiver goes to an ack ROUTER task (stdout lines
/// parsing as [`objectiveai_sdk::laboratories::daemon::HostStdioAck`]
/// are forwarded to the [`crate::context::LabHostStdio`] parked on the
/// resident entry; everything else is discarded as before). Also
/// returns whether THIS call spawned the child — the caller seeds the
/// dial list over stdio only then (a reused live child already has
/// its list; config changes reach it through the config handlers).
pub async fn spawn_leashed_until_ready_with_stdio(
    global: &crate::context::GlobalContext,
    key: &str,
    exe: &Path,
    configure: impl FnOnce(&mut Command),
) -> Result<(Option<String>, bool), crate::error::Error> {
    spawn_leashed_inner(global, key, exe, configure, true).await
}

/// The shared core of the two spawn entry points; `stdio` selects the
/// laboratory host's piped-stdin + ack-router mode. Returns the ready
/// address and whether a child was actually spawned (`false` = a live
/// resident child was reused).
async fn spawn_leashed_inner(
    global: &crate::context::GlobalContext,
    key: &str,
    exe: &Path,
    configure: impl FnOnce(&mut Command),
    stdio: bool,
) -> Result<(Option<String>, bool), crate::error::Error> {
    let gate = global.spawn_gate(key);
    let _guard = gate.lock().await;

    if let Some(address) = global.resident_child_address(key) {
        return Ok((address, false));
    }

    let name = exe
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string());

    let mut cmd = Command::new(exe);
    cmd.stdin(if stdio {
        std::process::Stdio::piped()
    } else {
        std::process::Stdio::null()
    })
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
    // Taken BEFORE the ready loop below: its select polls
    // `child.wait()`, and tokio's `wait()` takes-and-CLOSES the
    // child's stdin on its first poll (deadlock avoidance) — which
    // would both lose the handle and hand the host an immediate EOF
    // (its graceful-shutdown signal).
    let mut stdin = if stdio {
        Some(child.stdin.take().expect("stdin was piped"))
    } else {
        None
    };
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

    let stdio_handle = if let Some(stdin) = stdin.take() {
        // Ack ROUTER, replacing the discard drain: stdout lines that
        // parse as dial-list acks are forwarded to the LabHostStdio
        // parked below; everything else (stderr, stray output) is
        // consumed and discarded exactly as before, so the pipes stay
        // drained for the child's life.
        let (ack_tx, ack_rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            while let Some(event) = events.recv().await {
                if let crate::child_io::PipeEvent::Stdout(line) = event {
                    if let Some(ack) =
                        objectiveai_sdk::laboratories::daemon::parse_host_stdio_ack(&line)
                    {
                        if ack_tx.send(ack).is_err() {
                            // LabHostStdio dropped (child retired) —
                            // degrade to a plain drain.
                            break;
                        }
                    }
                }
            }
            while events.recv().await.is_some() {}
        });
        Some(std::sync::Arc::new(crate::context::LabHostStdio::new(
            stdin, ack_rx,
        )))
    } else {
        // Persistent drain: the child's std handles stay live for its
        // whole life — later writes (db's supervisor notices, api's
        // cost lines) are consumed and DISCARDED instead of blocking
        // the pipe or EPIPE-killing the server. Anything observable
        // goes through the server's published channel or the DB, per
        // the standing convention.
        tokio::spawn(async move { while events.recv().await.is_some() {} });
        None
    };

    global.hold_resident_child(key, child, address.clone(), stdio_handle);
    Ok((address, true))
}

/// Basename-or-path display name for spawn errors.
fn name_of(exe: &Path) -> String {
    exe.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| exe.display().to_string())
}

/// Stamp the context pair onto `cmd`'s env using the same env-var
/// names the [`crate::run::EnvConfigBuilder`] reads on the receiving
/// side. So a child cli (or any subprocess that uses the same
/// `Envconfig`-based loader) round-trips its parent's configuration
/// byte-identically: the GLOBAL half supplies the layout + author +
/// forbidden-flag keys, the SCOPED half the seven per-request identity
/// keys.
///
/// `Option`-typed fields are skipped on `None`, EXCEPT the five
/// per-request transient identity keys (`OBJECTIVEAI_AGENT_ID`,
/// `_FULL_ID`, `_REMOTE`, `_RESPONSE_ID`, `_RESPONSE_IDS`, and the
/// plugin caller trio `OBJECTIVEAI_PLUGIN_OWNER` / `_REPOSITORY` /
/// `_VERSION` — informational for the child (a plugin process can
/// learn its own coordinates); no reader turns it back into
/// identity, which is unspoofable by design), all `env_remove`'d on
/// `None` so the child cannot inherit a stale identity from the
/// parent's startup environment. Boolean fields are stamped only
/// when `true`.
pub fn apply_config_env(
    cmd: &mut Command,
    global: &crate::context::GlobalContext,
    scoped: &crate::context::ScopedContext,
) {
    if global.config_set_forbidden {
        cmd.env("CONFIG_SET_FORBIDDEN", "true");
    }
    if let Some(v) = global.objectiveai_dir.as_deref() {
        cmd.env("OBJECTIVEAI_DIR", v);
    }
    if let Some(v) = global.objectiveai_state.as_deref() {
        cmd.env("OBJECTIVEAI_STATE", v);
    }
    if let Some(v) = global.commit_author_name.as_deref() {
        cmd.env("COMMIT_AUTHOR_NAME", v);
    }
    if let Some(v) = global.commit_author_email.as_deref() {
        cmd.env("COMMIT_AUTHOR_EMAIL", v);
    }
    cmd.env(
        "OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY",
        scoped.agent_instance_hierarchy(),
    );
    match scoped.agent_id() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_ID");
        }
    }
    match scoped.agent_full_id() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_FULL_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_FULL_ID");
        }
    }
    match scoped.agent_remote() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_AGENT_REMOTE", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_AGENT_REMOTE");
        }
    }
    match scoped.response_id() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_RESPONSE_ID", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_RESPONSE_ID");
        }
    }
    match scoped.response_ids() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_RESPONSE_IDS", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_RESPONSE_IDS");
        }
    }
    // The PLUGIN CALLER identity trio — set on plugin children (whose
    // scope `plugins run` stamped via `with_plugin`), removed for
    // everyone else so a stale plugin identity can't leak through.
    match scoped.plugin_owner() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_OWNER", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_OWNER");
        }
    }
    match scoped.plugin_repository() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_REPOSITORY", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_REPOSITORY");
        }
    }
    match scoped.plugin_version() {
        Some(v) => {
            cmd.env("OBJECTIVEAI_PLUGIN_VERSION", v);
        }
        None => {
            cmd.env_remove("OBJECTIVEAI_PLUGIN_VERSION");
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
pub fn apply_daemon_env(cmd: &mut Command, global: &crate::context::GlobalContext) {
    cmd.env("ADDRESS", &global.daemon_bind_address);
    cmd.env("PORT", global.daemon_bind_port.to_string());
    match global.daemon_secret.as_deref() {
        Some(v) => {
            cmd.env("SECRET", v);
        }
        None => {
            cmd.env_remove("SECRET");
        }
    }
    match global.daemon_signature.as_deref() {
        Some(v) => {
            cmd.env("SIGNATURE", v);
        }
        None => {
            cmd.env_remove("SIGNATURE");
        }
    }
}
