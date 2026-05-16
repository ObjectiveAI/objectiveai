//! Validation / error-path checks for `/functions/inventions` and the
//! "completed state generates no completions" snapshot sanity test.

use objectiveai_sdk::functions::inventions::state::{
    AlphaScalarBranchState, AlphaScalarLeafState, AlphaVectorLeafState, ParamsState,
};

use crate::common::inventions::{make_request, params, post_expect_err, post_streaming, run_invention};

#[tokio::test]
async fn test_zero_leaf_width_rejected() {
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params("bad-zero", 0, 3, 5, 0, 0),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_zero_branch_width_rejected() {
    let request = make_request(
        ParamsState::AlphaScalarBranch(AlphaScalarBranchState {
            params: params("bad-zero-branch", 1, 0, 0, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        2,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_min_greater_than_max_rejected() {
    let request = make_request(
        ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
            params: params("bad-inverted", 0, 5, 3, 5, 3),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        3,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_completed_state_generates_no_completions() {
    let snapshot: serde_json::Value = serde_json::from_str(
        include_str!("../../assets/functions/inventions/client_tests/scalar_leaf_s42_0.json"),
    ).unwrap();
    let state: ParamsState = serde_json::from_value(snapshot["state"].clone()).unwrap();

    let request = make_request(state, 42);
    let result = run_invention(request).await;

    assert!(
        result.completions.is_empty(),
        "completed state should generate no completions, got {}",
        result.completions.len(),
    );
}

#[tokio::test]
async fn test_name_over_100_bytes_rejected() {
    let long_name = "a".repeat(101);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&long_name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_name_without_path_over_77_bytes_rejected() {
    let name = "a".repeat(78);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_name_without_path_at_77_bytes_accepted() {
    let name = "a".repeat(77);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let stream = post_streaming(request).await;
    assert!(stream.is_ok(), "name at exactly 77 bytes without path should be accepted");
}

#[tokio::test]
async fn test_name_with_valid_path_over_77_bytes_accepted() {
    let name = format!("{}-1", "a".repeat(80));
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let stream = post_streaming(request).await;
    assert!(stream.is_ok(), "name with valid path segment over 77 bytes should be accepted");
}

#[tokio::test]
async fn test_name_78_bytes_ending_in_dash_rejected() {
    let name = format!("{}-", "a".repeat(77));
    assert_eq!(name.len(), 78);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let _ = post_expect_err(request).await;
}

#[tokio::test]
async fn test_name_100_bytes_ending_in_dash_rejected() {
    let name = format!("{}-", "a".repeat(99));
    assert_eq!(name.len(), 100);
    let request = make_request(
        ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
            params: params(&name, 0, 3, 5, 3, 5),
            essay: None, input_schema: None, essay_tasks: None,
            tasks: None, tasks_length: None, description: None, readme: None, checker_seed: None,
        }),
        1,
    );
    let _ = post_expect_err(request).await;
}
