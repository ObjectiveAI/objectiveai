//! Test-fixture plugin that ATTACKS the daemon: it asks the host (via
//! the plugin Command protocol) to run `plugins daemon notify` against a
//! DIFFERENT plugin (`objectiveai/daemon-echo`). The host's notify
//! handler must reject this — a plugin may only notify itself
//! (`Error::DaemonNotifyNotSelf`). This fixture re-emits the rejection
//! as a `notification` on stdout so the `plugins run` stream surfaces it
//! for the e2e assertion.

use std::io::{BufRead, Write};

fn main() {
    let stdout = std::io::stdout();

    // Ask the host to notify a plugin that is NOT us.
    {
        let cmd = serde_json::json!({
            "type": "command",
            "id": "1",
            "command": [
                "plugins", "daemon", "notify",
                "--owner", "objectiveai",
                "--name", "daemon-echo",
                "--version", "0.0.1",
                "--input", "\"cross\"",
            ],
        });
        let mut handle = stdout.lock();
        let _ = writeln!(handle, "{cmd}");
        let _ = handle.flush();
    }

    // Read the host's response lines (written back to our stdin); surface
    // any error as a notification, stop at the command_complete marker.
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        let inner = &value["value"];
        match inner["type"].as_str() {
            Some("error") => {
                let note = serde_json::json!({
                    "type": "notification",
                    "notify_error": inner["message"].clone(),
                });
                let mut handle = stdout.lock();
                let _ = writeln!(handle, "{note}");
                let _ = handle.flush();
            }
            Some("command_complete") => break,
            _ => {}
        }
    }
}
