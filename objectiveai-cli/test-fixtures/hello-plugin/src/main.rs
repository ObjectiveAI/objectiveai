//! Test-fixture plugin. Reads argv[1] (or "anonymous"), emits one
//! `{"type":"notification","hello":"<arg>"}` line on stdout, exits 0.
//!
//! Used by `objectiveai-cli/tests/plugin_dispatch_e2e.rs` to verify
//! the cli's external-subcommand dispatch + spawn + JSONL round-trip
//! works end-to-end with a real on-disk binary.

use std::io::Write;

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| "anonymous".into());
    let line = format!(r#"{{"type":"notification","hello":"{arg}"}}"#);
    let stdout = std::io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(line.as_bytes());
    let _ = h.write_all(b"\n");
}
