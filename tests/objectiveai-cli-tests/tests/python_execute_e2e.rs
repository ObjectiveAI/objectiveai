//! End-to-end tests for `objectiveai.execute` in the embedded RustPython.
//!
//! Each test runs the `python` command whose code calls `objectiveai.execute`
//! to mutate host state in-process (apply a tag to a mock agent) and then looks
//! the tag back up, returning the lookup's `by` discriminator as the script's
//! value. A resolved tag → `"by": "tag"`; an unapplied one → `"by": "absent"`.
//! So asserting the returned value is `"tag"` proves `execute` actually ran the
//! command on the host AND returned native objects the script indexed into.
//!
//! Coverage: a single argv, a parallel batch (`list[list[str]]`), a recursive
//! single (python → execute → python → execute → apply), and a recursive batch
//! whose recursive arm is python → python → python → apply (three interpreter
//! levels), alongside a direct apply.

mod cli_test_util;

use objectiveai_sdk::cli::command::python::{Path as PyPath, Request as PyRequest, Response};

/// A mock agent spec, built inside the script with `json.dumps` so the test's
/// Rust source stays free of nested JSON quoting.
const SPEC: &str = r#"import json; spec = json.dumps({"upstream": "mock", "output_mode": "instruction"})"#;

fn py_request(code: impl Into<String>) -> PyRequest {
    PyRequest {
        path_type: PyPath::Python,
        code: code.into(),
        input: None,
        base: Default::default(),
    }
}

/// `objectiveai.execute(argv)` — single command. Apply a tag, look it up, return
/// the lookup's `by` (a single argv returns a one-element list, so `res[0]`).
#[tokio::test(flavor = "multi_thread")]
async fn execute_single() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let code = format!(
        r#"{SPEC}
objectiveai.execute(["agents", "tags", "apply", "--name", "exec-single", "--agent-inline", spec])
res = objectiveai.execute(["agents", "tags", "lookup", "--tag", "exec-single"])
res[0]["by"]"#
    );
    let response: Response = cli_test_util::execute_one(&executor, py_request(code)).await;
    assert_eq!(response, serde_json::json!("tag"), "single execute: {response:?}");
}

/// `objectiveai.execute(argv[])` — a parallel batch. Apply two tags in one
/// batch, look both up in another batch; a batch returns a list of lists, so
/// `res[i][0]`.
#[tokio::test(flavor = "multi_thread")]
async fn execute_batch() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    let code = format!(
        r#"{SPEC}
objectiveai.execute([
    ["agents", "tags", "apply", "--name", "exec-batch-a", "--agent-inline", spec],
    ["agents", "tags", "apply", "--name", "exec-batch-b", "--agent-inline", spec],
])
res = objectiveai.execute([
    ["agents", "tags", "lookup", "--tag", "exec-batch-a"],
    ["agents", "tags", "lookup", "--tag", "exec-batch-b"],
])
[res[0][0]["by"], res[1][0]["by"]]"#
    );
    let response: Response = cli_test_util::execute_one(&executor, py_request(code)).await;
    assert_eq!(response, serde_json::json!(["tag", "tag"]), "batch execute: {response:?}");
}

/// Recursive single: the script `execute`s a `python` command whose code itself
/// `execute`s the apply (python → python → apply). Then it looks the tag up.
#[tokio::test(flavor = "multi_thread")]
async fn execute_recursive_single() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // Inner python: apply the tag, then return "ok". `{:?}` debug-quotes it into
    // a python-compatible string literal (ASCII-only), avoiding nested quoting.
    let inner = format!(
        r#"{SPEC}; objectiveai.execute(["agents", "tags", "apply", "--name", "exec-rec-single", "--agent-inline", spec]); "ok""#
    );
    let code = format!(
        r#"objectiveai.execute(["python", "--code", {inner:?}])
res = objectiveai.execute(["agents", "tags", "lookup", "--tag", "exec-rec-single"])
res[0]["by"]"#
    );
    let response: Response = cli_test_util::execute_one(&executor, py_request(code)).await;
    assert_eq!(response, serde_json::json!("tag"), "recursive single execute: {response:?}");
}

/// Recursive batch: a batch whose arms are (a) a direct apply and (b) a python →
/// python → python → apply chain (three interpreter levels). Both tags must
/// resolve.
#[tokio::test(flavor = "multi_thread")]
async fn execute_recursive_batch() {
    let _base = cli_test_util::test_base_dir();
    let executor = cli_test_util::executor().await;

    // l3 applies the tag; l2 runs l3; l1 runs l2 — so the batch arm
    // `["python","--code", l1]` is python calling python calling python.
    let l3 = format!(
        r#"{SPEC}; objectiveai.execute(["agents", "tags", "apply", "--name", "exec-rec-batch-deep", "--agent-inline", spec]); "ok""#
    );
    let l2 = format!(r#"objectiveai.execute(["python", "--code", {l3:?}])"#);
    let l1 = format!(r#"objectiveai.execute(["python", "--code", {l2:?}])"#);
    let code = format!(
        r#"{SPEC}
objectiveai.execute([["agents", "tags", "apply", "--name", "exec-rec-batch-direct", "--agent-inline", spec], ["python", "--code", {l1:?}]])
res = objectiveai.execute([["agents", "tags", "lookup", "--tag", "exec-rec-batch-direct"], ["agents", "tags", "lookup", "--tag", "exec-rec-batch-deep"]])
[res[0][0]["by"], res[1][0]["by"]]"#
    );
    let response: Response = cli_test_util::execute_one(&executor, py_request(code)).await;
    assert_eq!(
        response,
        serde_json::json!(["tag", "tag"]),
        "recursive batch execute: {response:?}",
    );
}
