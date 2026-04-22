use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Once;

static BUILD_ONCE: Once = Once::new();

pub fn test_target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/test-cli")
}

pub fn cli_binary() -> PathBuf {
    let target_dir = test_target_dir();
    let mut path = target_dir.join("debug/objectiveai-cli");
    if cfg!(windows) {
        path.set_extension("exe");
    }

    BUILD_ONCE.call_once(|| {
        let status = Command::new("cargo")
            .args([
                "build", "-p", "objectiveai-cli",
                "--no-default-features", "--features", "rustpython",
                "--target-dir", target_dir.to_str().unwrap(),
            ])
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build failed");
    });

    path
}

/// CONFIG_BASE_DIR for the CLI under test.
///
/// Scoped to `tests/.objectiveai` so everything the CLI creates at runtime
/// (logs, cached function repos, filesystem config) lives under a single
/// gitignored directory that `test.sh` wipes on exit.
pub fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join(".objectiveai")
}

pub fn load_snapshot(dir: &Path, name: &str) -> serde_json::Value {
    let path = dir.join(format!("{name}.json"));
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read snapshot {}: {e}", path.display()));
    serde_json::from_str(&content).unwrap()
}

/// Round floats to 8 significant figures to match cross-language comparison.
pub fn rounded(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                let s12 = format!("{:.12e}", f);
                let f12: f64 = s12.parse().unwrap_or(f);
                let s8 = format!("{:.8e}", f12);
                let f8: f64 = s8.parse().unwrap_or(f12);
                serde_json::Value::Number(
                    serde_json::Number::from_f64(f8).unwrap_or_else(|| n.clone()),
                )
            } else {
                value.clone()
            }
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(rounded).collect())
        }
        serde_json::Value::Object(obj) => {
            serde_json::Value::Object(obj.iter().map(|(k, v)| (k.clone(), rounded(v))).collect())
        }
        _ => value.clone(),
    }
}

/// Run a CLI command and parse stdout as JSON.
pub fn run_cli(args: &[&str]) -> serde_json::Value {
    let mut cmd = Command::new(cli_binary());
    cmd.env("CONFIG_BASE_DIR", tests_dir());
    cmd.args(args);

    let output = cmd.output().expect("failed to execute CLI binary");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        panic!(
            "CLI exited with {}\nargs: {:?}\nstdout: {stdout}\nstderr: {stderr}",
            output.status, args
        );
    }

    let filtered: String = stdout.lines()
        .filter(|line| !line.starts_with("Logs ID: "))
        .collect::<Vec<_>>()
        .join("\n");
    serde_json::from_str(filtered.trim())
        .unwrap_or_else(|e| panic!("failed to parse CLI output as JSON: {e}\nstdout: {stdout}"))
}
