//! Test-fixture tool. Reads `MCP_SESSION_ID` from its env, increments
//! a per-session counter file at `<OBJECTIVEAI_STATE_DIR>/data/<session>.txt`,
//! and prints the new count to stdout.
//!
//! Used by the `agents_continuation_tool_session_e2e` snapshot test to
//! verify the cli forwards the session id into tool subprocesses and
//! that the count persists across continuation turns of the same
//! agent (same session) while staying independent across distinct
//! agents (different sessions).
//!
//! `OBJECTIVEAI_STATE_DIR` is provided by the host cli on every tool spawn
//! (`<dir>/state/<state>/tools/<owner>/<name>/<version>`) — the
//! tool's own install folder is committed and must never be written
//! to. Per-test-state isolation comes for free: each state gets its
//! own counter files.
//!
//! When MCP_SESSION_ID is unset, falls back to session `_default` so
//! the binary never errors — it may be deployed into a shared test
//! tools dir where other tests dispatch it without setting the env.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let session_id = std::env::var("MCP_SESSION_ID")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "_default".to_string());
    // Slash-containing session ids (e.g. lineage forms like
    // `cli/<chunk.id>`) would create subdirectories; flatten them
    // to keep one file per session.
    let safe_session = session_id.replace('/', "_");

    let data_dir = state_dir().join("data");
    if let Err(e) = fs::create_dir_all(&data_dir) {
        eprintln!("mkdir {}: {e}", data_dir.display());
        std::process::exit(1);
    }
    let count_path = data_dir.join(format!("{safe_session}.txt"));

    let prior: u64 = match fs::read_to_string(&count_path) {
        Ok(s) => s.trim().parse().unwrap_or(0),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => {
            eprintln!("read {}: {e}", count_path.display());
            std::process::exit(1);
        }
    };
    let next = prior + 1;

    let tmp_path = data_dir.join(format!("{safe_session}.txt.tmp"));
    {
        let mut f = match fs::File::create(&tmp_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("create {}: {e}", tmp_path.display());
                std::process::exit(1);
            }
        };
        if let Err(e) = writeln!(f, "{next}") {
            eprintln!("write {}: {e}", tmp_path.display());
            std::process::exit(1);
        }
    }
    if let Err(e) = fs::rename(&tmp_path, &count_path) {
        eprintln!(
            "rename {} -> {}: {e}",
            tmp_path.display(),
            count_path.display()
        );
        std::process::exit(1);
    }

    println!("{next}");
}

fn state_dir() -> PathBuf {
    PathBuf::from(
        std::env::var_os("OBJECTIVEAI_STATE_DIR").expect("OBJECTIVEAI_STATE_DIR is set by the host cli"),
    )
}
