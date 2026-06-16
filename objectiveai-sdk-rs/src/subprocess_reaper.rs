//! OS-level "die with the parent" leash for direct child processes.
//!
//! [`spawn`] launches a child that the **operating system** kills when
//! THIS process dies by any means — force-kill (`kill -9` /
//! `TerminateProcess`), crash, normal exit, or Ctrl+C — without relying
//! on Rust `Drop` (which does not run on `std::process::exit`, a panic
//! abort, or a force-kill). `kill_on_drop(true)` is still set as the
//! orderly fast-path, but the OS mechanism is the real guarantee.
//!
//! Mechanism per platform:
//!
//! - **Linux** — `prctl(PR_SET_PDEATHSIG, SIGKILL)` armed in `pre_exec`
//!   (post-fork, pre-exec), plus a `getppid()` recheck that aborts the
//!   exec if the parent already died in the fork→prctl window. This is
//!   race-free: from the instant the child execs, the kernel will SIGKILL
//!   it the moment the parent dies.
//! - **Windows** — the child is assigned to a process-wide singleton job
//!   object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. The job handle is
//!   leaked, so the kernel closes it exactly when this process dies
//!   (crash included), killing every assigned process. (Assignment
//!   happens just after spawn, matching the validated `objectiveai-db`
//!   path; the sub-millisecond spawn→assign window is the only gap.)
//! - **macOS** — no kernel parent-death primitive exists, so a tiny
//!   guardian process (this binary re-invoked with a hidden flag) watches
//!   both the parent and the child via kqueue `EVFILT_PROC`/`NOTE_EXIT`
//!   and SIGKILLs the child if the parent exits first. The guardian is
//!   detached (reparents to launchd) so it survives the parent's death.
//!   Residual gap: the child runs unguarded for the ~milliseconds it
//!   takes to launch + arm the guardian; a force-kill of the parent in
//!   that window can orphan that one child. (A truly atomic macOS leash
//!   would need `posix_spawn(POSIX_SPAWN_START_SUSPENDED)`, which `std`/
//!   `tokio` Command do not expose.)
//! - **Other Unix** — `kill_on_drop` only (best effort), matching the
//!   project's stance elsewhere.
//!
//! **Ctrl+C friendly:** the child is never detached from the console nor
//! moved to a new process group / session, so it still receives console
//! Ctrl+C naturally; the leash is the death backstop, not a replacement
//! for normal signal delivery.
//!
//! These children are expected to spawn no grandchildren of their own, so
//! only the direct child is reaped (the Windows job would reap a whole
//! tree regardless; Linux/macOS reap the direct child only).

// =====================================================================
// spawn — one definition per platform family
// =====================================================================

/// Spawn `cmd` leashed to the current process (see module docs). Sets
/// `kill_on_drop(true)` itself. The returned [`tokio::process::Child`] is
/// the live child; the OS leash is independent of that handle's lifetime.
#[cfg(target_os = "linux")]
pub fn spawn(
    cmd: &mut tokio::process::Command,
) -> std::io::Result<tokio::process::Child> {
    cmd.kill_on_drop(true);
    // Captured by value into the pre_exec closure; compared against
    // getppid() in the child to close the fork→prctl death race.
    let parent_pid = std::process::id();
    // SAFETY: the closure runs post-fork in the child, before exec.
    // `prctl` and `getppid` are async-signal-safe. PR_SET_PDEATHSIG arms
    // a SIGKILL delivered when the parent thread dies; the getppid check
    // aborts the exec if the parent already died (so we never run
    // orphaned).
    unsafe {
        cmd.pre_exec(move || {
            if nix::libc::prctl(nix::libc::PR_SET_PDEATHSIG, nix::libc::SIGKILL) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if nix::libc::getppid() != parent_pid as nix::libc::pid_t {
                return Err(std::io::Error::from_raw_os_error(nix::libc::ESRCH));
            }
            Ok(())
        });
    }
    cmd.spawn()
}

/// See the Linux variant. Windows assigns the child to a process-wide
/// kill-on-close job after spawn.
#[cfg(windows)]
pub fn spawn(
    cmd: &mut tokio::process::Command,
) -> std::io::Result<tokio::process::Child> {
    cmd.kill_on_drop(true);
    let mut child = cmd.spawn()?;
    if let Err(e) = assign_to_reaper_job(&child) {
        // Couldn't leash it — don't leave an unguarded child alive.
        let _ = child.start_kill();
        return Err(e);
    }
    Ok(child)
}

/// See the Linux variant. macOS launches a kqueue guardian after spawn.
#[cfg(target_os = "macos")]
pub fn spawn(
    cmd: &mut tokio::process::Command,
) -> std::io::Result<tokio::process::Child> {
    cmd.kill_on_drop(true);
    let child = cmd.spawn()?;
    let child_pid = match child.id() {
        Some(p) => p,
        // Already exited (impossible immediately after spawn, but be
        // safe): nothing to guard.
        None => return Ok(child),
    };
    let parent_pid = std::process::id();
    if let Err(e) = spawn_guardian(parent_pid, child_pid) {
        // No guardian → can't guarantee reaping. Kill the child rather
        // than leave it unleashed.
        // SAFETY: plain kill syscall on a pid we just spawned.
        unsafe {
            nix::libc::kill(child_pid as nix::libc::pid_t, nix::libc::SIGKILL);
        }
        return Err(e);
    }
    Ok(child)
}

/// Other Unix (BSDs etc.): `kill_on_drop` only — best effort, matching
/// the project's stance where no parent-death primitive is wired.
#[cfg(all(unix, not(target_os = "linux"), not(target_os = "macos")))]
pub fn spawn(
    cmd: &mut tokio::process::Command,
) -> std::io::Result<tokio::process::Child> {
    cmd.kill_on_drop(true);
    cmd.spawn()
}

// =====================================================================
// run_guardian_if_invoked — macOS guardian entry point; no-op elsewhere
// =====================================================================

/// Call as the FIRST statement of each binary's `main()`, before any
/// argument parsing.
///
/// On macOS, if this process was launched as a reaper guardian (the
/// hidden flag is the first argument), this runs the kqueue watch loop
/// and `process::exit`s — it never returns. On every other platform, and
/// for every normal invocation, it is a cheap no-op.
#[cfg(target_os = "macos")]
pub fn run_guardian_if_invoked() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() != Some(GUARDIAN_FLAG) {
        return;
    }
    let parent_pid = match args.next().and_then(|s| s.parse::<u32>().ok()) {
        Some(p) => p,
        None => std::process::exit(2),
    };
    let child_pid = match args.next().and_then(|s| s.parse::<u32>().ok()) {
        Some(p) => p,
        None => std::process::exit(2),
    };
    guardian_main(parent_pid, child_pid);
}

/// No-op on every non-macOS target (Linux/Windows use kernel primitives;
/// other Unix uses `kill_on_drop`). Present so callers can wire it
/// unconditionally into `main()`.
#[cfg(not(target_os = "macos"))]
pub fn run_guardian_if_invoked() {}

// =====================================================================
// Windows: process-wide kill-on-close job object
// =====================================================================

/// The single kill-on-close job for this process, created on first use.
/// Stored as `isize` because `HANDLE` (`*mut c_void`) is neither `Send`
/// nor `Sync`. `0` encodes "job creation failed" (a real job handle is
/// never null), in which case [`assign_to_reaper_job`] falls back to a
/// fresh per-child job so the leash still holds.
#[cfg(windows)]
static REAPER_JOB: std::sync::OnceLock<isize> = std::sync::OnceLock::new();

#[cfg(windows)]
fn assign_to_reaper_job(child: &tokio::process::Child) -> std::io::Result<()> {
    use windows_sys::Win32::System::JobObjects::AssignProcessToJobObject;

    let child_handle = child
        .raw_handle()
        .ok_or_else(|| std::io::Error::other("child has no process handle"))?;

    let job = match *REAPER_JOB.get_or_init(|| create_kill_on_close_job().unwrap_or(0)) {
        // Singleton creation failed earlier — fall back to a fresh
        // per-child job (leaks one handle, but the leash holds).
        0 => create_kill_on_close_job()?,
        j => j,
    };

    // SAFETY: `job` is a valid job handle and `child_handle` a valid
    // process handle; AssignProcessToJobObject is a plain Win32 call.
    if unsafe {
        AssignProcessToJobObject(
            job as *mut core::ffi::c_void,
            child_handle as *mut core::ffi::c_void,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

/// Create a job object whose closure kills every assigned process, and
/// return its handle as `isize`. The handle is intentionally NEVER
/// closed: the kernel closes it when this process dies, which fires
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Only that one flag is set — any
/// breakaway flag would let children escape the leash.
#[cfg(windows)]
fn create_kill_on_close_job() -> std::io::Result<isize> {
    use windows_sys::Win32::System::JobObjects::{
        CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    // SAFETY: plain Win32 calls with valid arguments; `info` is a
    // properly-sized, zero-initialized POD out-structure.
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const _ as *const core::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        Ok(job as isize)
    }
}

// =====================================================================
// macOS: kqueue guardian
// =====================================================================

/// Hidden first argument that marks a guardian re-invocation. Followed by
/// `<parent_pid> <child_pid>`.
#[cfg(target_os = "macos")]
const GUARDIAN_FLAG: &str = "--__objectiveai-subprocess-reaper-guardian";

/// Launch the guardian: this binary, re-invoked with the hidden flag and
/// the two pids, detached with null stdio. The returned child handle is
/// dropped immediately — the guardian must survive our death (it
/// reparents to launchd) so it can reap the leashed child afterward.
#[cfg(target_os = "macos")]
fn spawn_guardian(parent_pid: u32, child_pid: u32) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    std::process::Command::new(exe)
        .arg(GUARDIAN_FLAG)
        .arg(parent_pid.to_string())
        .arg(child_pid.to_string())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    Ok(())
}

/// Watch `parent_pid` and `child_pid` for exit via kqueue. If the parent
/// exits first, SIGKILL the child; if the child exits first, there is
/// nothing to do. Never returns.
#[cfg(target_os = "macos")]
fn guardian_main(parent_pid: u32, child_pid: u32) -> ! {
    use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};

    let kqueue = match Kqueue::new() {
        Ok(k) => k,
        Err(_) => std::process::exit(1),
    };
    let now = nix::libc::timespec { tv_sec: 0, tv_nsec: 0 };

    // Register the parent first. A registration error here means the
    // parent has ALREADY exited (e.g. ESRCH) — the leashed child must not
    // outlive it, so kill it and exit.
    let parent_reg = [KEvent::new(
        parent_pid as usize,
        EventFilter::EVFILT_PROC,
        EventFlag::EV_ADD | EventFlag::EV_ONESHOT,
        FilterFlag::NOTE_EXIT,
        0,
        0,
    )];
    if kqueue.kevent(&parent_reg, &mut [], Some(now)).is_err() {
        // SAFETY: plain kill syscall.
        unsafe {
            nix::libc::kill(child_pid as nix::libc::pid_t, nix::libc::SIGKILL);
        }
        std::process::exit(0);
    }

    // Register the child. If it's already gone (exited / PID recycled away
    // before we armed), there is nothing to guard.
    let child_reg = [KEvent::new(
        child_pid as usize,
        EventFilter::EVFILT_PROC,
        EventFlag::EV_ADD | EventFlag::EV_ONESHOT,
        FilterFlag::NOTE_EXIT,
        0,
        0,
    )];
    if kqueue.kevent(&child_reg, &mut [], Some(now)).is_err() {
        std::process::exit(0);
    }

    // Both armed — block until the first of the two exits.
    let mut events = [
        KEvent::new(0, EventFilter::EVFILT_PROC, EventFlag::empty(), FilterFlag::empty(), 0, 0),
        KEvent::new(0, EventFilter::EVFILT_PROC, EventFlag::empty(), FilterFlag::empty(), 0, 0),
    ];
    loop {
        match kqueue.kevent(&[], &mut events, None) {
            Ok(n) => {
                for ev in &events[..n] {
                    if ev.ident() == parent_pid as usize {
                        // Parent died → reap the child.
                        // SAFETY: plain kill syscall (no-op if already dead).
                        unsafe {
                            nix::libc::kill(child_pid as nix::libc::pid_t, nix::libc::SIGKILL);
                        }
                        std::process::exit(0);
                    }
                    if ev.ident() == child_pid as usize {
                        // Child exited on its own — nothing to reap.
                        std::process::exit(0);
                    }
                }
            }
            Err(nix::errno::Errno::EINTR) => continue,
            Err(_) => std::process::exit(1),
        }
    }
}
