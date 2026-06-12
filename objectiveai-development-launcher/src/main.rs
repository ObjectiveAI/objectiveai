//! Development launcher binary — pre-built-binary shim for the
//! objectiveai executables.
//!
//! Copies of this binary stand in for the five production
//! executables (`objectiveai{,-api,-viewer,-mcp,-db}.exe`). On
//! invocation each one identifies itself via
//! `std::env::current_exe()`, maps the filename stem to the matching
//! workspace binary, and runs the PRE-BUILT copy from the workspace's
//! `target/debug/` — propagating arguments, environment, stdio, and
//! exit code. No compilation happens here: build the binaries first
//! with the repo root's `test-build.sh` (each suite's `test.sh` runs
//! it automatically). Keeping cargo out of the shims means concurrent
//! test processes can never trigger rebuilds or relink races against
//! binaries that are currently running.
//!
//! Repo-root resolution, in order:
//! 1. **Committed-shim layout** (the copies in the repo's
//!    `.objectiveai/bin/`): the workspace root is two directories up
//!    from the shim (`<repo>/.objectiveai/bin/x.exe` → `<repo>`),
//!    detected by the presence of `Cargo.toml` there. The committed
//!    exe is built WITHOUT a baked path, so it is machine-independent
//!    and byte-identical across clones.
//! 2. **Dev-install layout** (`~/.objectiveai/` copies placed by
//!    `install.sh`): falls back to the repo root baked in at build
//!    time via `OBJECTIVEAI_REPO_ROOT`.

const BAKED_REPO_ROOT: Option<&str> = option_env!("OBJECTIVEAI_REPO_ROOT");

/// Clear `HANDLE_FLAG_INHERIT` on this process's std handles (Windows).
///
/// When an invoker captures this shim's output through pipes
/// (`.output()`, `$(...)`, test harnesses), the pipe write-ends arrive
/// as this process's std handles with the inherit flag SET. Spawning
/// the real binary with `bInheritHandles=TRUE` would then hand ORPHAN
/// copies of those pipe handles to the child — and onward through
/// every descendant, including detached servers that outlive the whole
/// chain. A long-lived server silently holding the invoker's pipe
/// write-end means the invoker's read-to-EOF never completes: it hangs
/// until the server dies. Clearing the flag here stops the cascade at
/// the first hop. The child still gets working stdio — Rust's std
/// duplicates the parent's std handles into fresh inheritable copies
/// for `Stdio::inherit()` regardless of the originals' flag.
#[cfg(windows)]
fn clear_stdio_inheritance() {
    use std::os::windows::io::AsRawHandle;

    const HANDLE_FLAG_INHERIT: u32 = 0x0000_0001;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn SetHandleInformation(
            handle: *mut core::ffi::c_void,
            mask: u32,
            flags: u32,
        ) -> i32;
    }

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    for handle in [
        stdin.as_raw_handle(),
        stdout.as_raw_handle(),
        stderr.as_raw_handle(),
    ] {
        // Best-effort: a null handle (stream not attached) is skipped;
        // a console handle that rejects the call is harmless — a
        // console doesn't pin a pipe open.
        if !handle.is_null() {
            unsafe {
                SetHandleInformation(handle, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

fn binary_for_stem(stem: &str) -> Option<&'static str> {
    match stem {
        "objectiveai" => Some("objectiveai-cli"),
        "objectiveai-api" => Some("objectiveai-api"),
        "objectiveai-viewer" => Some("objectiveai-viewer"),
        "objectiveai-mcp" => Some("objectiveai-mcp"),
        "objectiveai-db" => Some("objectiveai-db"),
        _ => None,
    }
}

/// `<repo>` such that `<repo>/Cargo.toml` is the workspace manifest:
/// two levels above the shim when it lives at
/// `<repo>/.objectiveai/bin/`, else the baked dev-install root.
fn repo_root(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Some(root) = exe.parent().and_then(|bin| bin.parent()).and_then(|d| d.parent()) {
        if root.join("Cargo.toml").is_file() {
            return Some(root.to_path_buf());
        }
    }
    BAKED_REPO_ROOT.map(std::path::PathBuf::from)
}

fn main() -> std::process::ExitCode {
    #[cfg(windows)]
    clear_stdio_inheritance();

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("objectiveai-development-launcher: current_exe() failed: {e}");
            return std::process::ExitCode::from(127);
        }
    };
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let bin = match binary_for_stem(&stem) {
        Some(b) => b,
        None => {
            eprintln!(
                "objectiveai-development-launcher: unknown launcher name {stem:?} \
                 (expected one of: objectiveai, objectiveai-api, \
                 objectiveai-viewer, objectiveai-mcp, objectiveai-db)"
            );
            return std::process::ExitCode::from(2);
        }
    };
    let Some(root) = repo_root(&exe) else {
        eprintln!(
            "objectiveai-development-launcher: cannot locate the workspace root \
             (no Cargo.toml two levels above {exe:?} and no baked OBJECTIVEAI_REPO_ROOT)"
        );
        return std::process::ExitCode::from(127);
    };
    let target = root
        .join("target")
        .join("debug")
        .join(format!("{bin}{}", std::env::consts::EXE_SUFFIX));
    if !target.is_file() {
        eprintln!(
            "objectiveai-development-launcher: {} is not built; run \
             `bash {}/test-build.sh` first",
            target.display(),
            root.display(),
        );
        return std::process::ExitCode::from(127);
    }

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    // Caller CWD and environment are deliberately inherited — the
    // shim is transparent apart from the path indirection.
    match std::process::Command::new(&target).args(&args).status() {
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            std::process::ExitCode::from(code.clamp(0, 255) as u8)
        }
        Err(e) => {
            eprintln!(
                "objectiveai-development-launcher: failed to run {}: {e}",
                target.display(),
            );
            std::process::ExitCode::from(127)
        }
    }
}
