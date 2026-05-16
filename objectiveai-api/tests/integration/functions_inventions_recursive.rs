//! Recursive function-invention snapshot tests (depth 0/1/2/3) plus
//! initial-state validation checks for the recursive client. ~84
//! tests.

use crate::{recursive_test_3x, recursive_test_3x_unrouted};
use objectiveai_sdk::functions::inventions::state::{
    AlphaScalarBranchState, AlphaScalarLeafState, AlphaScalarState,
    AlphaVectorBranchState, AlphaVectorLeafState, AlphaVectorState,
    ParamsState,
};

use crate::common::inventions::{
    invalid_scalar_leaf_task, invalid_scalar_schema, make_recursive_request,
    normalize_recursive, params, post_recursive_expect_err,
    run_recursive_invention, valid_scalar_leaf_task, valid_scalar_schema,
    valid_vector_leaf_task, valid_vector_schema,
};

// ---------------------------------------------------------------------------
// Depth-0 leaf snapshot tests (just 2; recursive is wasteful at d0)
// ---------------------------------------------------------------------------

recursive_test_3x!(test_scalar_leaf_d0,
    AlphaScalarLeaf, AlphaScalarLeafState,
    "rsl-baseline", 0, 1, 1, 2, 4, 100,
    "scalar_leaf_d0");

recursive_test_3x!(test_vector_leaf_d0,
    AlphaVectorLeaf, AlphaVectorLeafState,
    "rvl-baseline", 0, 1, 1, 2, 4, 200,
    "vector_leaf_d0");

// ---------------------------------------------------------------------------
// Depth-1 scalar (diverse widths and configs)
// ---------------------------------------------------------------------------

recursive_test_3x!(test_scalar_d1_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-min", 1, 1, 1, 1, 1, 1000,
    "scalar_d1_min");

recursive_test_3x!(test_scalar_d1_default,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-default", 1, 3, 5, 3, 5, 1100,
    "scalar_d1_default");

recursive_test_3x!(test_scalar_d1_narrow_branch_wide_leaf,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-nbwl", 1, 1, 2, 6, 8, 1200,
    "scalar_d1_narrow_branch_wide_leaf");

recursive_test_3x!(test_scalar_d1_wide_branch_narrow_leaf,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-wbnl", 1, 6, 8, 1, 2, 1300,
    "scalar_d1_wide_branch_narrow_leaf");

recursive_test_3x!(test_scalar_d1_exact_4,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d1-exact4", 1, 4, 4, 4, 4, 1400,
    "scalar_d1_exact_4");

recursive_test_3x_unrouted!(test_scalar_d1_unrouted,
    AlphaScalar, AlphaScalarState,
    "rs-d1-unrouted", 1, 2, 3, 2, 3, 1500,
    "scalar_d1_unrouted");

// ---------------------------------------------------------------------------
// Depth-1 vector (diverse widths and configs)
// ---------------------------------------------------------------------------

recursive_test_3x!(test_vector_d1_min,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-min", 1, 1, 1, 1, 1, 2000,
    "vector_d1_min");

recursive_test_3x!(test_vector_d1_default,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-default", 1, 3, 5, 3, 5, 2100,
    "vector_d1_default");

recursive_test_3x!(test_vector_d1_wide_branch,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-wb", 1, 5, 8, 1, 2, 2200,
    "vector_d1_wide_branch");

recursive_test_3x!(test_vector_d1_narrow_branch,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-nb", 1, 1, 2, 5, 8, 2300,
    "vector_d1_narrow_branch");

recursive_test_3x!(test_vector_d1_exact_3,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-exact3", 1, 3, 3, 3, 3, 2400,
    "vector_d1_exact_3");

recursive_test_3x_unrouted!(test_vector_d1_unrouted,
    AlphaVector, AlphaVectorState,
    "rv-d1-unrouted", 1, 2, 4, 2, 4, 2500,
    "vector_d1_unrouted");

recursive_test_3x!(test_vector_d1_wide_range,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d1-range", 1, 1, 10, 1, 10, 2600,
    "vector_d1_wide_range");

// ---------------------------------------------------------------------------
// Depth-2 scalar and vector
// ---------------------------------------------------------------------------

recursive_test_3x!(test_scalar_d2_narrow,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d2-narrow", 2, 2, 3, 2, 3, 3000,
    "scalar_d2_narrow");

recursive_test_3x!(test_scalar_d2_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d2-min", 2, 1, 1, 1, 1, 3100,
    "scalar_d2_min");

recursive_test_3x!(test_vector_d2_default,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d2-default", 2, 3, 5, 3, 5, 3200,
    "vector_d2_default");

recursive_test_3x!(test_vector_d2_narrow,
    AlphaVectorBranch, AlphaVectorBranchState,
    "rvb-d2-narrow", 2, 2, 3, 2, 3, 3300,
    "vector_d2_narrow");

recursive_test_3x_unrouted!(test_scalar_d2_unrouted,
    AlphaScalar, AlphaScalarState,
    "rs-d2-unrouted", 2, 1, 2, 1, 2, 3400,
    "scalar_d2_unrouted");

// ---------------------------------------------------------------------------
// Depth-3 (just one, expensive)
// ---------------------------------------------------------------------------

recursive_test_3x!(test_scalar_d3_min,
    AlphaScalarBranch, AlphaScalarBranchState,
    "rsb-d3-min", 3, 1, 1, 1, 1, 4000,
    "scalar_d3_min");

// ---------------------------------------------------------------------------
// Initial-state validation tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_recursive_invalid_scalar_input_schema() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-bad-schema", 0, 1, 1, 2, 4),
        essay: Some("An essay about feelings.".to_string()),
        input_schema: Some(invalid_scalar_schema()),
        essay_tasks: None,
        tasks: None,
        tasks_length: None,
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5000);
    let err = post_recursive_expect_err(request).await;
    assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
}

#[tokio::test]
async fn test_recursive_valid_schema_invalid_tasks_scalar_leaf() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-bad-tasks", 0, 1, 1, 2, 4),
        essay: Some("An essay about scoring sentiment.".to_string()),
        input_schema: Some(valid_scalar_schema()),
        essay_tasks: Some("Tasks for scoring.".to_string()),
        tasks: Some(vec![invalid_scalar_leaf_task(), invalid_scalar_leaf_task()]),
        tasks_length: Some(2),
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5200);
    let err = post_recursive_expect_err(request).await;
    assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
}

#[tokio::test]
async fn test_recursive_valid_schema_valid_tasks_scalar_leaf() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-good-sl", 0, 1, 1, 2, 4),
        essay: None,
        input_schema: Some(valid_scalar_schema()),
        essay_tasks: Some("Good tasks incoming.".to_string()),
        tasks: Some(vec![valid_scalar_leaf_task(), valid_scalar_leaf_task()]),
        tasks_length: Some(2),
        description: Some("A valid scalar function.".to_string()),
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5300);
    let result = normalize_recursive(run_recursive_invention(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    crate::common::inventions::assert_recursive_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_schema_valid_tasks_scalar_leaf.json"),
        include_str!("../../assets/functions/inventions/recursive_client_tests/valid_schema_valid_tasks_scalar_leaf.json"),
    );
}

#[tokio::test]
async fn test_recursive_valid_vector_schema_valid_tasks() {
    let state = ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
        params: params("inv-good-vl", 0, 1, 1, 2, 4),
        essay: Some("Ranking things.".to_string()),
        input_schema: Some(valid_vector_schema()),
        essay_tasks: None,
        tasks: Some(vec![valid_vector_leaf_task(), valid_vector_leaf_task()]),
        tasks_length: Some(2),
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5400);
    let result = normalize_recursive(run_recursive_invention(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    crate::common::inventions::assert_recursive_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_vector_schema_valid_tasks.json"),
        include_str!("../../assets/functions/inventions/recursive_client_tests/valid_vector_schema_valid_tasks.json"),
    );
}

#[tokio::test]
async fn test_recursive_predicted_tasks_length_too_low() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-tl-low", 0, 1, 1, 2, 4),
        essay: Some("Writing an essay.".to_string()),
        input_schema: None,
        essay_tasks: Some("Tasks essay.".to_string()),
        tasks: None,
        tasks_length: Some(0),
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5500);
    let err = post_recursive_expect_err(request).await;
    assert!(
        err.contains("tasks_length") && err.contains("outside bounds"),
        "expected tasks_length bounds error, got: {err}",
    );
}

#[tokio::test]
async fn test_recursive_predicted_tasks_length_too_high() {
    let state = ParamsState::AlphaVectorLeaf(AlphaVectorLeafState {
        params: params("inv-tl-high", 0, 1, 1, 2, 4),
        essay: None,
        input_schema: Some(valid_vector_schema()),
        essay_tasks: None,
        tasks: None,
        tasks_length: Some(99),
        description: Some("Description present.".to_string()),
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5600);
    let err = post_recursive_expect_err(request).await;
    assert!(
        err.contains("tasks_length") && err.contains("outside bounds"),
        "expected tasks_length bounds error, got: {err}",
    );
}

#[tokio::test]
async fn test_recursive_predicted_tasks_length_too_high_branch() {
    let state = ParamsState::AlphaScalarBranch(AlphaScalarBranchState {
        params: params("inv-tl-branch", 1, 3, 5, 2, 4),
        essay: Some("Branch essay.".to_string()),
        input_schema: Some(valid_scalar_schema()),
        essay_tasks: Some("Branch tasks essay.".to_string()),
        tasks: None,
        tasks_length: Some(50),
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5700);
    let err = post_recursive_expect_err(request).await;
    assert!(
        err.contains("tasks_length") && err.contains("outside bounds"),
        "expected tasks_length bounds error, got: {err}",
    );
}

#[tokio::test]
async fn test_recursive_predicted_tasks_length_below_branch_min() {
    let state = ParamsState::AlphaVectorBranch(AlphaVectorBranchState {
        params: params("inv-tl-vb-low", 1, 3, 8, 2, 4),
        essay: None,
        input_schema: None,
        essay_tasks: Some("Tasks essay for vector branch.".to_string()),
        tasks: None,
        tasks_length: Some(1),
        description: Some("Vector branch description.".to_string()),
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5800);
    let err = post_recursive_expect_err(request).await;
    assert!(
        err.contains("tasks_length") && err.contains("outside bounds"),
        "expected tasks_length bounds error, got: {err}",
    );
}

#[tokio::test]
async fn test_recursive_valid_schema_no_tasks_with_essay() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-schema-only", 0, 1, 1, 2, 4),
        essay: Some("A great essay about things.".to_string()),
        input_schema: Some(valid_scalar_schema()),
        essay_tasks: None,
        tasks: None,
        tasks_length: None,
        description: None,
        readme: None,
        checker_seed: None,
    });
    let request = make_recursive_request(state, 5900);
    let result = normalize_recursive(run_recursive_invention(request).await);
    let json = serde_json::to_string_pretty(&result).unwrap();
    crate::common::inventions::assert_recursive_snapshot(
        &json,
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/functions/inventions/recursive_client_tests/valid_schema_no_tasks_with_essay.json"),
        include_str!("../../assets/functions/inventions/recursive_client_tests/valid_schema_no_tasks_with_essay.json"),
    );
}

#[tokio::test]
async fn test_recursive_invalid_schema_with_tasks_and_description() {
    let state = ParamsState::AlphaScalarLeaf(AlphaScalarLeafState {
        params: params("inv-full-bad", 0, 1, 1, 2, 4),
        essay: Some("An elaborate essay.".to_string()),
        input_schema: Some(invalid_scalar_schema()),
        essay_tasks: Some("Essay about tasks.".to_string()),
        tasks: Some(vec![invalid_scalar_leaf_task()]),
        tasks_length: Some(1),
        description: Some("A complete but invalid function.".to_string()),
        readme: Some("# README\nThis is invalid.".to_string()),
        checker_seed: None,
    });
    let request = make_recursive_request(state, 6000);
    let err = post_recursive_expect_err(request).await;
    assert!(err.contains("invalid_state"), "expected invalid_state error, got: {err}");
}
