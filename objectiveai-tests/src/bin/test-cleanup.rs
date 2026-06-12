//! `test-cleanup` — reset the repo's shared test `OBJECTIVEAI_DIR`.
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
//! 3. Deletes `state/` (and the runtime `bin/locks/`) with retries —
//!    Windows releases dead processes' file handles asynchronously.
//!
//! Invoked by the repo-root `test-cleanup.sh`, which every suite's
//! `test.sh` runs at start and end unless
//! `OBJECTIVEAI_TESTS_RUNNING_FROM_ROOT` says the root `test.sh`
//! already brackets the whole run.

use std::path::{Path, PathBuf};

fn main() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent");
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

    remove_with_retry(&root.join("state"));
    remove_with_retry(&root.join("bin").join("locks"));
    println!(
        "test-cleanup: {} lockfile(s) inspected, {killed} process(es) killed, state/ cleared",
        gates.len()
    );
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
