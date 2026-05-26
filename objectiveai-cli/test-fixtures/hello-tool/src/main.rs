//! Test-fixture tool. Writes two lines to stdout and exits 0. Used
//! by `objectiveai-cli/tests/tool_dispatch_e2e.rs` to verify the
//! cli's `tools <name>` dispatch + ToolLine notification + exit-code
//! propagation work end-to-end with a real on-disk executable.

fn main() {
    println!("hello, world");
    println!("hi again");
}
