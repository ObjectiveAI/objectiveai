//! Test-fixture plugin. Writes three known lines to STDERR and exits 0
//! — nothing on stdout. Used by
//! `objectiveai-cli/tests/plugin_logs_e2e.rs` to verify that
//! `plugins run` captures a plugin's stderr into
//! `objectiveai.plugin_messages`, readable via `plugins logs list`.

fn main() {
    eprintln!("stderr-plugin line 1");
    eprintln!("stderr-plugin line 2");
    eprintln!("stderr-plugin line 3");
}
