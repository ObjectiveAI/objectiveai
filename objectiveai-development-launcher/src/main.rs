//! Development launcher binary — `cargo run` shim for the
//! objectiveai executables.
//!
//! Copies of this binary stand in for the five production
//! executables (`objectiveai{,-api,-viewer,-mcp,-db}.exe`). On
//! invocation each one identifies itself via
//! `std::env::current_exe()`, maps the filename stem to the matching
//! workspace package, and shells out to
//! `cargo run -q -p <pkg> --manifest-path <repo>/Cargo.toml -- <args>`
//! — propagating arguments, environment, stdio, and exit code, so
//! edits to any crate flow through immediately with no install step.
//!
//! Repo-root resolution, in order:
//! 1. **Committed-shim layout** (the copies in the repo's
//!    `.objectiveai/bin/`): the workspace root is two directories up
//!    from the shim (`<repo>/.objectiveai/bin/x.exe` → `<repo>`),
//!    detected by the presence of `Cargo.toml` there. The committed
//!    exe is built WITHOUT a baked path, so it is machine-independent
//!    and byte-identical across clones.
//! 2. **Dev-install layout** (`~/.objectiveai/` copies placed by
//!    `install.sh`): falls back to the repo root baked in at build
//!    time via `OBJECTIVEAI_REPO_ROOT`.

const BAKED_REPO_ROOT: Option<&str> = option_env!("OBJECTIVEAI_REPO_ROOT");

fn package_for_stem(stem: &str) -> Option<&'static str> {
    match stem {
        "objectiveai" => Some("objectiveai-cli"),
        "objectiveai-api" => Some("objectiveai-api"),
        "objectiveai-viewer" => Some("objectiveai-viewer"),
        "objectiveai-mcp" => Some("objectiveai-mcp"),
        "objectiveai-db" => Some("objectiveai-db"),
        _ => None,
    }
}

/// `<repo>` such that `<repo>/Cargo.toml` is the workspace manifest:
/// two levels above the shim when it lives at
/// `<repo>/.objectiveai/bin/`, else the baked dev-install root.
fn repo_root(exe: &std::path::Path) -> Option<std::path::PathBuf> {
    if let Some(root) = exe.parent().and_then(|bin| bin.parent()).and_then(|d| d.parent()) {
        if root.join("Cargo.toml").is_file() {
            return Some(root.to_path_buf());
        }
    }
    BAKED_REPO_ROOT.map(std::path::PathBuf::from)
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
                 objectiveai-viewer, objectiveai-mcp, objectiveai-db)"
            );
            return std::process::ExitCode::from(2);
        }
    };
    let Some(root) = repo_root(&exe) else {
        eprintln!(
            "objectiveai-development-launcher: cannot locate the workspace root \
             (no Cargo.toml two levels above {exe:?} and no baked OBJECTIVEAI_REPO_ROOT)"
        );
        return std::process::ExitCode::from(127);
    };
    let manifest = root.join("Cargo.toml");

    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();

    // Caller CWD and environment are deliberately inherited — the
    // shim is transparent apart from the cargo indirection.
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("run")
        .arg("-q")
        .arg("-p")
        .arg(pkg)
        .arg("--manifest-path")
        .arg(&manifest);
    if !args.is_empty() {
        cmd.arg("--");
        cmd.args(&args);
    }

    match cmd.status() {
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            std::process::ExitCode::from(code.clamp(0, 255) as u8)
        }
        Err(e) => {
            eprintln!(
                "objectiveai-development-launcher: failed to spawn `cargo run -q -p {pkg}` \
                 against {manifest:?}: {e}"
            );
            std::process::ExitCode::from(127)
        }
    }
}
