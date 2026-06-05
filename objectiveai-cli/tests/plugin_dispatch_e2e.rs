//! End-to-end test: a fixture plugin binary is copied into a temp
//! plugins dir and dispatched via `objectiveai-cli plugins <name> <args>`.
//! We assert the host's JSONL output contains the expected re-emitted
//! notification between the `begin` / `end` markers.

mod cli_test_util;

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Build the `hello-plugin` workspace member and return the path to
/// its compiled binary. Cargo doesn't expose `CARGO_BIN_EXE_*` for
/// sibling workspace crates, so we invoke `cargo build` explicitly
/// and locate the artifact under the workspace target dir (respecting
/// `CARGO_TARGET_DIR` if set).
fn build_and_locate_hello_plugin() -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(&cargo)
        .args(["build", "-p", "hello-plugin"])
        .status()
        .expect("failed to spawn cargo build");
    assert!(status.success(), "cargo build -p hello-plugin failed");

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let bin = target.join("debug").join(if cfg!(windows) {
        "hello-plugin.exe"
    } else {
        "hello-plugin"
    });
    assert!(bin.exists(), "hello-plugin binary missing at {bin:?}");
    bin
}

#[test]
fn hello_plugin_dispatch_produces_expected_output() {
    let base = cli_test_util::test_base_dir();
    let plugins_dir = base.join("plugins");
    std::fs::create_dir_all(&plugins_dir).unwrap();

    let fixture = build_and_locate_hello_plugin();
    let plugin_subdir = plugins_dir.join("hello");
    std::fs::create_dir_all(&plugin_subdir).unwrap();
    let target = plugin_subdir.join(if cfg!(windows) {
        "plugin.exe"
    } else {
        "plugin"
    });
    std::fs::copy(&fixture, &target).expect("failed to copy fixture binary");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let cli = cli_test_util::cli_binary();
    let output = Command::new(cli)
        .env("CONFIG_BASE_DIR", &base)
        .args(["plugins", "run", "hello", "world"])
        .output()
        .expect("failed to run cli");

    assert!(
        output.status.success(),
        "cli exited {:?}; stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    // Each cli stdout line is the leaf `plugins::run::ResponseItem`
    // serialized at the wire. Every `cli/command` aggregator
    // `Response`/`ResponseItem` is `#[serde(untagged)]` (sdk commit
    // 39c3320e7), so the root `RunItem::Command(_)`,
    // `cli::command::ResponseItem`, and `plugins::ResponseItem`
    // wrappers all collapse. The leaf `plugins::run::ResponseItem`
    // is itself untagged with variants `Mcp { type: "mcp", url }`,
    // `Error { type: "error", ... }`, and `Notification(Value)` —
    // so a notification carrying the plugin's raw payload
    // `{"hello":"world"}` lands on the wire as bare `{"hello":"world"}`,
    // not wrapped in any `Plugins/Run/` envelope.
    let stdout = String::from_utf8(output.stdout).expect("cli stdout not utf-8");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        !lines.is_empty(),
        "expected at least one notification, got: {lines:?}"
    );

    let hello_count = lines
        .iter()
        .filter(|line| {
            let Ok(v) = serde_json::from_str::<Value>(line) else {
                return false;
            };
            v.pointer("/hello") == Some(&Value::String("world".into()))
        })
        .count();
    assert_eq!(
        hello_count, 1,
        "expected exactly one hello/world notification in {lines:?}"
    );

}
