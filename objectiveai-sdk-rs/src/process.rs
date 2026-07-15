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


/// Send a terminate to `pid` — Unix `SIGTERM`, Windows `TerminateProcess`.
/// Returns 1 if a live process with that pid was signalled, 0 otherwise
/// (no such process, or the signal/open failed). Idempotent and
/// best-effort: killing an already-dead pid is a 0, not an error.
#[cfg(unix)]
pub fn kill_pid(pid: u32) -> usize {
    // SAFETY: `kill(2)` with a valid signal number is sound — it only
    // checks the target's existence and posts a signal, touching no
    // memory. `SIGTERM` to a nonexistent pid returns -1 (ESRCH), which
    // we map to 0.
    let rc = unsafe { nix::libc::kill(pid as nix::libc::pid_t, nix::libc::SIGTERM) };
    if rc == 0 { 1 } else { 0 }
}

#[cfg(windows)]
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
