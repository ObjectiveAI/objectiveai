"""HTTP integration tests for function executions."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3

from objectiveai_sdk.functions.executions.http import create_function_execution
from objectiveai_sdk.functions.executions.response.streaming import FunctionExecutionChunk
from tests.http_test_util import HttpTestCase, http_test_suite, ASSETS_DIR


def mock_remote(name: str) -> dict:
    return {"remote": "mock", "name": name}


globals().update(http_test_suite(
    name="function executions http",
    fn=create_function_execution,
    snapshots_dir=ASSETS_DIR / "functions" / "executions" / "client_tests",
    chunk_cls=FunctionExecutionChunk,
    chunk_to_unary=objectiveai_sdk_pyo3.function_execution_chunk_to_unary,
    normalize=objectiveai_sdk_pyo3.normalize_function_execution_for_tests,
    cases=[
        HttpTestCase(
            snapshot="mock_1_scalar_leaf_binary_seed_42",
            body={
                "function": mock_remote("binary-classifier"),
                "profile": mock_remote("solo-instruction"),
                "input": {"text": "Hello world"},
                "seed": 42,
            },
        ),
        HttpTestCase(
            snapshot="mock_7_vector_5_criteria_seed_42",
            body={
                "function": mock_remote("five-criteria-ranker"),
                "profile": mock_remote("schema-heavy-trio"),
                "input": {"items": ["Option A", "Option B", "Option C"]},
                "seed": 42,
            },
        ),
        HttpTestCase(
            snapshot="mock_20_vector_super_branch_seed_42",
            body={
                "function": mock_remote("nested-vector-super-branch"),
                "profile": mock_remote("nested-vector-inline-remote"),
                "input": {"items": ["Alpha", "Beta", "Gamma"]},
                "seed": 42,
            },
        ),
    ],
))
