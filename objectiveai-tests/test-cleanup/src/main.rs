//! `objectiveai-test-cleanup` — reset the repo's shared test
//! `OBJECTIVEAI_DIR`.
//!
//! 1. Walks `<repo>/.objectiveai` for every SDK lockfile gate
//!    (`*.lock`, skipping the paired `*.live.lock` announces) —
//!    `bin/locks/`, `state/*/locks/`, and the per-agent trees under
//!    `state/*/locks/agents/{instances/**,tags}/`.
//! 2. Resolves each lock's live owner PIDs via
//!    [`objectiveai_sdk::lockfile::owners`] and kills them — the api
//!    server, per-state db supervisors (taking their postmasters
//!    with them), viewers, mcp servers, and any agent-holding cli
//!    processes left over from a previous run.
//! 3. Sweeps every remaining process whose executable lives under
//!    `<repo>/target/` — leaked children that own no lockfile
//!    (plugin RMCP servers orphaned by a hard-killed cli, fixture
//!    tools). On Windows a running exe can't be replaced, so a
//!    single leak would fail the next `test-build.sh` relink with
//!    "Access is denied".
//! 4. Deletes `state/` (and the runtime `bin/locks/`) with retries —
//!    Windows releases dead processes' file handles asynchronously.
//!    Skipped when `OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY` is set
//!    (steps 1–3 only), leaving the run's db data on disk so a later
//!    `objectiveai` invocation can re-spawn the db and read it.
//!
//! Invoked (as the pre-built `target/debug` binary — `test-build.sh`
//! compiles it) by the repo-root `test-cleanup.sh`, which every
//! suite's `test.sh` runs at start and end unless
//! `OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT` says the root `test.sh`
//! already brackets the whole run.

use std::path::{Path, PathBuf};

fn main() {
    // `OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY` (set non-empty) skips the
    // `state/` wipe (step 4 below): kill everything, but leave the
    // on-disk per-state db data so a later `objectiveai` run can
    // re-spawn against it and read what the tests wrote. An env var
    // (not a flag) so it rides process inheritance through the shell
    // wrappers without each layer having to forward argv.
    let kill_only = std::env::var_os("OBJECTIVEAI_TEST_CLEANUP_KILL_ONLY")
        .is_some_and(|v| !v.is_empty());

    // <repo>/objectiveai-tests/test-cleanup → <repo>.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crate dir is two levels under the repo root");
    let root = repo.join(".objectiveai");
    if !root.is_dir() {
        println!("test-cleanup: {} does not exist; nothing to do", root.display());
        return;
    }

    let mut gates: Vec<PathBuf> = Vec::new();
    collect_gates(&root, &mut gates);

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let mut killed = 0usize;
    for gate in &gates {
        let Some(dir) = gate.parent() else { continue };
        let Some(stem) = gate.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let key = filename_unescape(stem.trim_end_matches(".lock"));
        let pids = match runtime.block_on(objectiveai_sdk::lockfile::owners(dir, &key)) {
            Ok(pids) => pids,
            Err(e) => {
                eprintln!("test-cleanup: owners({}, {key:?}): {e}", dir.display());
                continue;
            }
        };
        for pid in pids {
            if pid == std::process::id() || pid == 0 {
                continue;
            }
            killed += kill_pid(pid, gate);
        }
    }

    killed += kill_target_processes(&repo.join("target"));

    // Kill-only: stop here with `state/` and `bin/locks/` intact. The
    // stale locks are harmless — the SDK lockfile gate re-acquires
    // once it sees the recorded owner is no longer alive — so the next
    // `objectiveai` invocation re-spawns the db against the preserved
    // data dirs.
    if kill_only {
        println!(
            "test-cleanup: {} lockfile(s) inspected, {killed} process(es) killed, state/ preserved (kill-only)",
            gates.len()
        );
        return;
    }

    remove_with_retry(&root.join("state"));
    remove_with_retry(&root.join("bin").join("locks"));
    println!(
        "test-cleanup: {} lockfile(s) inspected, {killed} process(es) killed, state/ cleared",
        gates.len()
    );
}

/// Lowercase + forward-slash normalization for prefix comparison.
/// Windows paths compare case-insensitively, and the two sides here
/// arrive with DIFFERENT separators: the compile-time
/// `CARGO_MANIFEST_DIR` is baked with forward slashes when cargo is
/// invoked from an MSYS shell (`test-build.sh`), while sysinfo
/// reports executable paths with backslashes.
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().to_lowercase().replace('\\', "/")
}

/// Kill every live process (except this one) whose executable path
/// is under `<repo>/target/` — repo-built binaries that leaked past
/// their lockfile-owning parents.
fn kill_target_processes(target: &Path) -> usize {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System, UpdateKind};
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_exe(UpdateKind::Always),
    );
    let me = std::process::id();
    // Match BOTH the literal path and its canonical resolution: the
    // repo's `target/` may be a junction onto another volume (a
    // relocated target dir), and sysinfo reports executables by
    // their REAL path on the target volume while cargo and the
    // shims speak the junction view.
    let mut prefixes = vec![normalize_path(target)];
    if let Ok(real) = std::fs::canonicalize(target) {
        let real = normalize_path(&real);
        let real = real
            .strip_prefix("//?/")
            .map(str::to_string)
            .unwrap_or(real);
        if !prefixes.contains(&real) {
            prefixes.push(real);
        }
    }
    let mut killed = 0usize;
    for (pid, process) in sys.processes() {
        if pid.as_u32() == me {
            continue;
        }
        let Some(exe) = process.exe() else { continue };
        let exe_normalized = normalize_path(exe);
        if !prefixes.iter().any(|p| exe_normalized.starts_with(p.as_str())) {
            continue;
        }
        println!(
            "test-cleanup: killing pid {} ({:?}) running {}",
            pid.as_u32(),
            process.name(),
            exe.display(),
        );
        let _ = process
            .kill_with(Signal::Term)
            .or_else(|| Some(process.kill()));
        killed += 1;
    }
    killed
}

/// Recursively collect SDK lockfile GATE files: names ending `.lock`
/// but not `.live.lock` (the announce of the same claim).
fn collect_gates(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_gates(&path, out);
        } else if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if name.ends_with(".lock") && !name.ends_with(".live.lock") {
                out.push(path);
            }
        }
    }
}

/// Inverse of the SDK lockfile's `filename_escape`: `[A-Za-z0-9_-]`
/// bytes pass through; everything else was written as `%XX`
/// (uppercase hex). Malformed escapes pass through verbatim —
/// `owners` on a nonsense key is just an empty result.
fn filename_unescape(escaped: &str) -> String {
    let bytes = escaped.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1)
                .and_then(|b| (*b as char).to_digit(16))
                .zip(bytes.get(i + 2).and_then(|b| (*b as char).to_digit(16)));
            if let Some((hi, lo)) = hex {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// SIGTERM (Unix) / TerminateProcess (Windows) one pid. Returns 1 if
/// a live process existed and was targeted.
fn kill_pid(pid: u32, gate: &Path) -> usize {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, Signal, System};
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    match sys.process(sysinfo::Pid::from_u32(pid)) {
        Some(process) => {
            println!(
                "test-cleanup: killing pid {pid} ({:?}) holding {}",
                process.name(),
                gate.display()
            );
            let _ = process
                .kill_with(Signal::Term)
                .or_else(|| Some(process.kill()));
            1
        }
        None => 0,
    }
}

/// `remove_dir_all` with widening backoff — dead processes' handles
/// release asynchronously on Windows (AV/indexers can also hold
/// files briefly).
fn remove_with_retry(dir: &Path) {
    if !dir.exists() {
        return;
    }
    for (attempt, delay_ms) in [0u64, 250, 1000, 3000, 8000].into_iter().enumerate() {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
        match std::fs::remove_dir_all(dir) {
            Ok(()) => return,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
            Err(e) => {
                if attempt == 4 {
                    eprintln!(
                        "test-cleanup: failed to remove {} after retries: {e}",
                        dir.display()
                    );
                }
            }
        }
    }
}
