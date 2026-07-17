//! Process-management helpers that pair with [`crate::lockfile`]'s owner
//! lookups: given a pid (e.g. from [`crate::lockfile::owners`] /
//! [`crate::lockfile::owners_in_tree`]), ask the OS to terminate it —
//! plus the repo-wide [`no_window`] spawn hygiene.

/// Windows `CREATE_NO_WINDOW`: spawn the child without a console
/// window. Our services are windowless (daemon, laboratory host, db
/// supervisor, the viewer shell), so every console-subsystem child
/// they spawn — podman, postgres, plugins, tools, runners, the cli
/// re-execing itself — otherwise allocates and FLASHES its own
/// console at the user. Applied at every runtime spawn site in the
/// repository; no-op off Windows.
#[cfg(any(
    feature = "http",
    feature = "mcp",
    feature = "cli-executor",
    feature = "cli-listener",
    feature = "lockfile",
    feature = "subprocess-reaper"
))]
pub fn no_window(command: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    {
        let _ = command;
    }
}


/// The one-line stdout readiness handshake every persistent server the
/// daemon spawns (db / api / mcp / viewer / laboratory host) emits the
/// moment it is serving: `{"type":"ready","address":...}`. The daemon —
/// which holds the child's stdout pipe for its whole life (the children
/// are OS-leashed to it) — reads lines until this parses, caches the
/// address, and keeps draining. `address` is the server's client
/// connect coordinate (`http://…` for api/mcp, `postgresql://…` for
/// db) or `None` for servers with no listener (viewer, laboratory
/// host — they dial out; the line is pure readiness).
///
/// This replaced the old lockfile-readiness scheme: servers no longer
/// publish locks (the daemon is their sole spawner and owns their
/// lifetime), so there is nothing on disk to discover. The daemon's
/// OWN lock is the one survivor — short-lived CLI processes need a
/// rendezvous outside any pipe to find an already-running daemon.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename = "ready")]
pub struct ServerReady {
    pub address: Option<String>,
}

/// Print the readiness line to stdout and flush. UNCONDITIONAL by
/// contract — never behind a suppress-output flag: the spawner blocks
/// on this line, and the daemon's persistent drain makes any later
/// stray writes harmless.
pub fn print_ready(address: Option<&str>) {
    use std::io::Write;
    let line = serde_json::to_string(&ServerReady {
        address: address.map(str::to_string),
    })
    .expect("ServerReady serializes");
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{line}");
    let _ = stdout.flush();
}

/// Parse one stdout line as the readiness handshake. `None` for
/// anything else (servers may print other lines; the reader skips
/// them).
pub fn parse_ready(line: &str) -> Option<ServerReady> {
    serde_json::from_str(line.trim()).ok()
}

/// Send a terminate to `pid` — Unix `SIGTERM`, Windows `TerminateProcess`.
/// Returns 1 if a live process with that pid was signalled, 0 otherwise
/// (no such process, or the signal/open failed). Idempotent and
/// best-effort: killing an already-dead pid is a 0, not an error.
///
/// Gated on the two features that carry the nix / windows-sys deps —
/// the module itself exists for every subprocess-spawning feature
/// (see the `no_window` gate).
#[cfg(all(unix, any(feature = "lockfile", feature = "subprocess-reaper")))]
pub fn kill_pid(pid: u32) -> usize {
    // SAFETY: `kill(2)` with a valid signal number is sound — it only
    // checks the target's existence and posts a signal, touching no
    // memory. `SIGTERM` to a nonexistent pid returns -1 (ESRCH), which
    // we map to 0.
    let rc = unsafe { nix::libc::kill(pid as nix::libc::pid_t, nix::libc::SIGTERM) };
    if rc == 0 { 1 } else { 0 }
}

#[cfg(all(windows, any(feature = "lockfile", feature = "subprocess-reaper")))]
pub fn kill_pid(pid: u32) -> usize {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_TERMINATE, TerminateProcess,
    };
    // SAFETY: `OpenProcess` returns null on failure (no such pid, or no
    // rights) — we check before use. `TerminateProcess` and `CloseHandle`
    // operate only on the handle we just opened; the handle is closed on
    // every path.
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if handle.is_null() {
            return 0;
        }
        let ok = TerminateProcess(handle, 1);
        CloseHandle(handle);
        if ok != 0 { 1 } else { 0 }
    }
}
