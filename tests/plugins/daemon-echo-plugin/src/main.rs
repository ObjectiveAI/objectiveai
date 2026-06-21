//! Test-fixture DAEMON plugin (`daemon: true`). The plugin daemon
//! launches it as `<exec> daemon begin`; the daemon bridges each
//! `plugins daemon notify` input into this process's stdin as one JSON
//! line. Every line received is written, verbatim, to
//! `$OBJECTIVEAI_STATE_DIR/input.json`, so the e2e test can read it back
//! and assert delivery. Stays resident until stdin EOF.

use std::io::BufRead;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != ["daemon", "begin"] {
        // Only the daemon entrypoint is supported.
        std::process::exit(2);
    }
    let state_dir = std::env::var("OBJECTIVEAI_STATE_DIR")
        .expect("OBJECTIVEAI_STATE_DIR is set by the daemon");
    let path = std::path::Path::new(&state_dir).join("input.json");

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let _ = std::fs::write(&path, line.as_bytes());
    }
}
