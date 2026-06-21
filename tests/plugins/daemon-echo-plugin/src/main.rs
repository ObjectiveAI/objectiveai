//! Test-fixture DAEMON plugin (`daemon: true`). The plugin daemon runs
//! it as `<exec> daemon begin` under the SHARED plugin executor, so it
//! has the full bidirectional protocol. On startup it:
//!   1. emits a nested `agents tags apply` command to tag a mock agent,
//!   2. waits for the host's `command_complete`,
//!   3. records the applied tag name in `$OBJECTIVEAI_STATE_DIR/input.json`
//!      (the "echo file" the e2e test reads),
//!   4. stays resident until stdin EOF (the daemon kills it on exit).

use std::io::{BufRead, Write};

const TAG: &str = "daemon-applied-tag";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args != ["daemon", "begin"] {
        std::process::exit(2);
    }
    let state_dir = std::env::var("OBJECTIVEAI_STATE_DIR")
        .expect("OBJECTIVEAI_STATE_DIR is set by the daemon");

    // 1. Ask the host to apply a tag to an inline mock agent.
    let command = serde_json::json!({
        "type": "command",
        "id": "1",
        "command": [
            "agents", "tags", "apply",
            "--name", TAG,
            "--agent-inline", "{\"upstream\":\"mock\",\"output_mode\":\"instruction\"}",
        ],
    });
    {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let _ = writeln!(handle, "{command}");
        let _ = handle.flush();
    }

    // 2. Wait for the host's command_complete before recording.
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    for line in lines.by_ref() {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        if value["value"]["type"] == "command_complete" {
            break;
        }
    }

    // 3. Record the applied tag so the e2e test can read it.
    let _ = std::fs::write(
        std::path::Path::new(&state_dir).join("input.json"),
        serde_json::to_string(TAG).unwrap(),
    );

    // 4. Stay resident until the daemon dies (stdin EOF).
    for _ in lines {}
}
