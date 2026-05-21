//! Development launcher binary for `~/.objectiveai/`.
//!
//! Copies of this binary live at the install paths for the four
//! production executables (`objectiveai{,-api,-viewer,-mcp}.exe`).
//! On invocation each one identifies itself via
//! `std::env::current_exe()`, maps the filename stem to the matching
//! workspace package, and shells out to `cargo run -p <pkg> --
//! <forwarded args>` against the local repo (path baked in at build
//! time via `env!("OBJECTIVEAI_REPO_ROOT")`).
//!
//! Used as a development aid so edits to any of the four crates
//! flow through immediately without re-running the per-crate
//! `install.sh`.

const REPO_ROOT: &str = env!("OBJECTIVEAI_REPO_ROOT");

fn package_for_stem(stem: &str) -> Option<&'static str> {
    match stem {
        "objectiveai" => Some("objectiveai-cli"),
        "objectiveai-api" => Some("objectiveai-api"),
        "objectiveai-viewer" => Some("objectiveai-viewer"),
        "objectiveai-mcp" => Some("objectiveai-mcp-cli"),
        _ => None,
    }
}

fn main() -> std::process::ExitCode {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("objectiveai-development-launcher: current_exe() failed: {e}");
            return std::process::ExitCode::from(127);
        }
    };
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let pkg = match package_for_stem(&stem) {
        Some(p) => p,
        None => {
            eprintln!(
                "objectiveai-development-launcher: unknown launcher name {stem:?} \
                 (expected one of: objectiveai, objectiveai-api, \
                 objectiveai-viewer, objectiveai-mcp)"
            );
            return std::process::ExitCode::from(2);
        }
    };

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run").arg("-p").arg(pkg);
    if !args.is_empty() {
        cmd.arg("--");
        cmd.args(&args);
    }
    cmd.current_dir(REPO_ROOT);

    match cmd.status() {
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            std::process::ExitCode::from(code.clamp(0, 255) as u8)
        }
        Err(e) => {
            eprintln!(
                "objectiveai-development-launcher: failed to spawn `cargo run -p {pkg}` \
                 in {REPO_ROOT:?}: {e}"
            );
            std::process::ExitCode::from(127)
        }
    }
}
