fn main() {
    set_stack_size();
    claude_agent_sdk_runner();

    #[cfg(feature = "orchestrator-bollard")]
    laboratories_local();
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

#[cfg(feature = "orchestrator-bollard")]
fn laboratories_local() {
    // The MCP binary is always linux-musl (for Docker container injection),
    // regardless of the platform the API is built on.
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let musl_target = format!("{arch}-unknown-linux-musl");
    let profile = std::env::var("PROFILE").unwrap();

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = std::path::Path::new(&manifest_dir).parent().unwrap();

    let validate_script = workspace_dir.join("objectiveai-mcp").join("validate.sh");
    let mut args: Vec<&str> = vec!["--target", &musl_target];
    if profile == "release" {
        args.push("--release");
    }
    let output = run_bash(&validate_script, &args);

    assert!(
        output.status.success(),
        "objectiveai-mcp/validate.sh failed:\n{}\n{}Run: bash objectiveai-mcp/build.sh --target {musl_target}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        if profile == "release" { " --release" } else { "" }
    );

    let binary_path = workspace_dir
        .join("objectiveai-mcp")
        .join("embed")
        .join(&musl_target)
        .join(&profile)
        .join("objectiveai-mcp");

    println!(
        "cargo:rustc-env=OBJECTIVEAI_MCP_BINARY_PATH={}",
        binary_path.display()
    );
    println!("cargo:rerun-if-changed=../objectiveai-mcp/embed/");
}

fn claude_agent_sdk_runner() {
    let target = std::env::var("TARGET").unwrap();
    let profile = std::env::var("PROFILE").unwrap();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_dir = std::path::Path::new(&manifest_dir).parent().unwrap();

    #[cfg(feature = "claude-agent-sdk-javascript")]
    {
        let validate_script = workspace_dir
            .join("objectiveai-claude-agent-sdk-runner-js")
            .join("validate.sh");
        let mut args: Vec<&str> = vec!["--target", &target];
        if profile == "release" {
            args.push("--release");
        }
        let output = run_bash(&validate_script, &args);
        assert!(
            output.status.success(),
            "objectiveai-claude-agent-sdk-runner-js/validate.sh failed:\n{}\n{}Run: bash objectiveai-claude-agent-sdk-runner-js/build.sh --target {target}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            if profile == "release" { " --release" } else { "" }
        );
        let binary_name = if target.contains("windows") {
            "objectiveai-claude-agent-sdk-runner-js.exe"
        } else {
            "objectiveai-claude-agent-sdk-runner-js"
        };
        let binary_path = workspace_dir
            .join("objectiveai-claude-agent-sdk-runner-js")
            .join("embed")
            .join(&target)
            .join(&profile)
            .join(binary_name);
        println!(
            "cargo:rustc-env=OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_JS_PATH={}",
            binary_path.display()
        );
        println!("cargo:rerun-if-changed=../objectiveai-claude-agent-sdk-runner-js/embed/");
    }

    #[cfg(feature = "claude-agent-sdk-python")]
    {
        let validate_script = workspace_dir
            .join("objectiveai-claude-agent-sdk-runner-py")
            .join("validate.sh");
        let mut args: Vec<&str> = vec!["--target", &target];
        if profile == "release" {
            args.push("--release");
        }
        let output = run_bash(&validate_script, &args);
        assert!(
            output.status.success(),
            "objectiveai-claude-agent-sdk-runner-py/validate.sh failed:\n{}\n{}Run: bash objectiveai-claude-agent-sdk-runner-py/build.sh --target {target}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            if profile == "release" { " --release" } else { "" }
        );
        let binary_name = if target.contains("windows") {
            "objectiveai-claude-agent-sdk-runner-py.exe"
        } else {
            "objectiveai-claude-agent-sdk-runner-py"
        };
        let binary_path = workspace_dir
            .join("objectiveai-claude-agent-sdk-runner-py")
            .join("embed")
            .join(&target)
            .join(&profile)
            .join(binary_name);
        println!(
            "cargo:rustc-env=OBJECTIVEAI_CLAUDE_AGENT_SDK_RUNNER_PY_PATH={}",
            binary_path.display()
        );
        println!("cargo:rerun-if-changed=../objectiveai-claude-agent-sdk-runner-py/embed/");
    }
}
