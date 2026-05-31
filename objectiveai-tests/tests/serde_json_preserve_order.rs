//! Enforces that every tracked `Cargo.toml` declaring a `serde_json` dependency
//! enables the `preserve_order` feature. `preserve_order` is load-bearing
//! repo-wide (content-addressed IDs, prefix-tree assembly, JSON round-trips).
//!
//! File enumeration uses the git index, so gitignored paths are naturally
//! excluded.

use std::path::PathBuf;

use git2::Repository;
use toml::Value;

#[test]
fn every_cargo_toml_with_serde_json_enables_preserve_order() {
    let repo = Repository::discover(env!("CARGO_MANIFEST_DIR")).expect("discover git repo");
    let workdir = repo.workdir().expect("repo has a workdir").to_path_buf();
    let index = repo.index().expect("repo has an index");

    let mut cargo_tomls: Vec<String> = index
        .iter()
        .filter_map(|entry| {
            let path = String::from_utf8(entry.path).ok()?;
            let basename = PathBuf::from(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .map(str::to_string)?;
            (basename == "Cargo.toml").then_some(path)
        })
        .collect();
    cargo_tomls.sort();
    assert!(!cargo_tomls.is_empty(), "no Cargo.toml files found in git index");

    let manifests: Vec<(String, Value)> = cargo_tomls
        .iter()
        .map(|rel| {
            let full = workdir.join(rel);
            let src = std::fs::read_to_string(&full)
                .unwrap_or_else(|e| panic!("read {}: {}", full.display(), e));
            let value: Value = toml::from_str(&src)
                .unwrap_or_else(|e| panic!("parse {}: {}", full.display(), e));
            (rel.clone(), value)
        })
        .collect();

    let workspace_root_compliant = manifests.iter().any(|(_, manifest)| {
        let Some(deps) = manifest
            .get("workspace")
            .and_then(Value::as_table)
            .and_then(|ws| ws.get("dependencies"))
            .and_then(Value::as_table)
        else {
            return false;
        };
        deps.get("serde_json")
            .map(|spec| check_serde_json(spec).is_ok())
            .unwrap_or(false)
    });

    let mut violations: Vec<String> = Vec::new();
    for (rel, manifest) in &manifests {
        for (location, deps) in dep_tables(manifest) {
            let Some(spec) = deps.get("serde_json") else { continue };
            if is_workspace_inherited(spec) {
                if !workspace_root_compliant {
                    violations.push(format!(
                        "{rel} [{location}] serde_json = {{ workspace = true }} \
                         but [workspace.dependencies].serde_json does not enable preserve_order"
                    ));
                }
                continue;
            }
            if let Err(reason) = check_serde_json(spec) {
                violations.push(format!("{rel} [{location}] {reason}"));
            }
        }
    }

    if !violations.is_empty() {
        violations.sort();
        panic!(
            "{} serde_json preserve_order violation(s):\n  {}",
            violations.len(),
            violations.join("\n  ")
        );
    }
}

fn dep_tables(manifest: &Value) -> Vec<(String, &toml::map::Map<String, Value>)> {
    let mut out = Vec::new();
    let Some(root) = manifest.as_table() else { return out };

    for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(t) = root.get(kind).and_then(Value::as_table) {
            out.push((kind.to_string(), t));
        }
    }

    if let Some(targets) = root.get("target").and_then(Value::as_table) {
        for (cfg, target_val) in targets {
            let Some(target) = target_val.as_table() else { continue };
            for kind in ["dependencies", "dev-dependencies", "build-dependencies"] {
                if let Some(t) = target.get(kind).and_then(Value::as_table) {
                    out.push((format!("target.{cfg}.{kind}"), t));
                }
            }
        }
    }

    if let Some(ws_deps) = root
        .get("workspace")
        .and_then(Value::as_table)
        .and_then(|ws| ws.get("dependencies"))
        .and_then(Value::as_table)
    {
        out.push(("workspace.dependencies".to_string(), ws_deps));
    }

    out
}

fn is_workspace_inherited(spec: &Value) -> bool {
    spec.as_table()
        .and_then(|t| t.get("workspace"))
        .and_then(Value::as_bool)
        == Some(true)
}

fn check_serde_json(spec: &Value) -> Result<(), &'static str> {
    match spec {
        Value::String(_) => Err("serde_json declared as bare version (missing preserve_order)"),
        Value::Table(t) => {
            let features = t.get("features").and_then(Value::as_array);
            match features {
                Some(arr) if arr.iter().any(|v| v.as_str() == Some("preserve_order")) => Ok(()),
                Some(_) => Err("serde_json features list does not include preserve_order"),
                None => Err("serde_json table has no features list (missing preserve_order)"),
            }
        }
        _ => Err("serde_json has unexpected value form"),
    }
}
