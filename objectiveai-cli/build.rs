fn main() {
    set_stack_size();
    #[cfg(feature = "viewer")]
    embed_viewer();
}

/// Set the main thread stack size to 16 MB for all supported platforms.
fn set_stack_size() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    let flag = match (os.as_str(), env.as_str()) {
        ("windows", "msvc") => "/STACK:16777216",
        ("windows", "gnu") => "-Wl,--stack,16777216",
        ("macos", _) | ("ios", _) => "-Wl,-stack_size,0x1000000",
        ("linux", _) | ("freebsd", _) | ("netbsd", _) | ("openbsd", _) | ("dragonfly", _) => {
            "-Wl,-z,stacksize=16777216"
        }
        _ => return,
    };

    println!("cargo:rustc-link-arg={flag}");
}

/// Runs a bash script cross-platform.
/// On Windows, uses `cmd /c bash` so that cmd.exe resolves bash from PATH
/// (finding Git bash instead of WSL bash).
fn run_bash(script: &std::path::Path, args: &[&str]) -> std::process::Output {
    if cfg!(windows) {
        let mut cmd_args = vec!["/c", "bash", script.to_str().unwrap()];
        cmd_args.extend(args);
        std::process::Command::new("cmd")
            .args(&cmd_args)
            .output()
            .expect("Failed to run bash script")
    } else {
        std::process::Command::new("bash")
            .arg(script)
            .args(args)
            .output()
            .expect("Failed to run bash script")
    }
}

#[cfg(feature = "viewer")]
fn embed_viewer() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = std::path::Path::new(&manifest_dir).parent().unwrap();
    let target = std::env::var("TARGET").unwrap();
    let profile = std::env::var("PROFILE").unwrap();

    let validate_script = workspace_dir.join("objectiveai-viewer").join("validate.sh");
    let mut args: Vec<&str> = vec!["--target", &target];
    if profile == "release" {
        args.push("--release");
    }
    let output = run_bash(&validate_script, &args);

    assert!(
        output.status.success(),
        "objectiveai-viewer/validate.sh failed:\n{}\n{}Run: bash objectiveai-viewer/build.sh --target {target}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        if profile == "release" { " --release" } else { "" }
    );

    let binary_name = if target.contains("windows") {
        "objectiveai-viewer.exe"
    } else {
        "objectiveai-viewer"
    };

    let binary_path = workspace_dir
        .join("objectiveai-viewer")
        .join("embed")
        .join(&target)
        .join(&profile)
        .join(binary_name);

    println!(
        "cargo:rustc-env=OBJECTIVEAI_VIEWER_BINARY_PATH={}",
        binary_path.display()
    );
    println!("cargo:rerun-if-changed=../objectiveai-viewer/embed/");
}
