//! Test-fixture tool. Writes one line to stdout, one to stderr,
//! then exits with code 7. Exercises ToolLine stream tagging +
//! exit-code propagation through the cli's `tools <name>` dispatch.

fn main() {
    println!("wrote to stdout");
    eprintln!("wrote to stderr");
    std::process::exit(7);
}
