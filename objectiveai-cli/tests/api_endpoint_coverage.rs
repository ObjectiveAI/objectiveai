//! Coverage tests for `objectiveai-cli/src/api/...` leaves:
//!
//! - `every_api_route_has_a_cli_leaf`: every `.route(...)` in
//!   `objectiveai-api/src/run.rs` must have a matching leaf file under
//!   `objectiveai-cli/src/api/...`. Convention: `<METHOD> /<a>/<b>/<c>` maps
//!   to `<a>/<b>/<c>/<method>.rs`, lowercased verb (`post`, `get`, `delete`).
//!
//! - `every_api_leaf_flattens_agent_id_arg`: every cli leaf that calls
//!   `call_unary` / `call_streaming` must flatten the shared `AgentIdArg`
//!   struct so the `--agent-id` flag is uniformly available on every
//!   endpoint.

fn api_routes() -> Vec<(String, String)> {
    let run_rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("objectiveai-api")
        .join("src")
        .join("run.rs");
    let src = std::fs::read_to_string(&run_rs_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", run_rs_path.display()));

    // Captures every `.route("<path>", axum::routing::<method>(...)` declaration.
    let re = regex::Regex::new(
        r#"\.route\(\s*"([^"]+)"\s*,\s*(?:axum::routing::)?(get|post|delete|put)\("#,
    )
    .expect("regex compiles");

    re.captures_iter(&src)
        .map(|cap| (cap.get(1).unwrap().as_str().to_string(), cap.get(2).unwrap().as_str().to_string()))
        .collect()
}

fn leaf_rel_path(path: &str, method: &str) -> std::path::PathBuf {
    let segments: Vec<&str> = path
        .trim_start_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    let mut rel = std::path::PathBuf::from("api");
    for seg in &segments {
        rel.push(seg);
    }
    rel.push(format!("{method}.rs"));
    rel
}

#[test]
fn every_api_route_has_a_cli_leaf() {
    let cli_src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut missing: Vec<String> = Vec::new();

    for (path, method) in api_routes() {
        let rel = leaf_rel_path(&path, &method);
        let abs = cli_src_dir.join(&rel);
        if !abs.exists() {
            missing.push(format!("{} {} -> src/{}", method.to_uppercase(), path, rel.display()));
        }
    }

    assert!(
        missing.is_empty(),
        "uncovered API endpoints (each route in objectiveai-api/src/run.rs needs a matching CLI leaf):\n  {}",
        missing.join("\n  "),
    );
}

/// Every CLI leaf under `src/api/<...>/<method>.rs` that corresponds to a
/// real API route must flatten the shared `AgentIdArg` struct on its
/// `Args` — i.e. contain `pub agent_id: crate::api::agent_id_arg::AgentIdArg`.
///
/// This is structural (line-level) rather than full-syn-AST because the
/// existing sibling test is already line-level; both rely on the
/// codebase's consistent formatting.
#[test]
fn every_api_leaf_flattens_agent_id_arg() {
    let cli_src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    const MARKER: &str = "pub agent_id: crate::api::agent_id_arg::AgentIdArg";

    let mut missing: Vec<String> = Vec::new();

    for (path, method) in api_routes() {
        let rel = leaf_rel_path(&path, &method);
        let abs = cli_src_dir.join(&rel);
        let Ok(src) = std::fs::read_to_string(&abs) else {
            // Leaf doesn't even exist — the other test already covers that.
            continue;
        };
        if !src.contains(MARKER) {
            missing.push(format!("{} {} -> src/{}", method.to_uppercase(), path, rel.display()));
        }
    }

    assert!(
        missing.is_empty(),
        "API leaves missing `{}` flatten (every endpoint must accept `--agent-id`):\n  {}",
        MARKER,
        missing.join("\n  "),
    );
}
