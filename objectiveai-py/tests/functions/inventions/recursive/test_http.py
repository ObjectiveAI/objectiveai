"""HTTP integration tests for recursive function inventions."""

import objectiveai._pyo3 as objectiveai_pyo3

from objectiveai.functions.inventions.recursive.http import create_function_invention_recursive
from objectiveai.functions.inventions.recursive.response.streaming import FunctionInventionRecursiveChunk
from tests.http_test_util import HttpTestCase, http_test_suite, ASSETS_DIR

MOCK_INVENTION_AGENT = {"upstream": "mock", "output_mode": "instruction", "mode": "invention"}

globals().update(http_test_suite(
    name="recursive function inventions http",
    fn=create_function_invention_recursive,
    snapshots_dir=ASSETS_DIR / "functions" / "inventions" / "recursive_client_tests",
    chunk_cls=FunctionInventionRecursiveChunk,
    chunk_to_unary=objectiveai_pyo3.function_invention_recursive_chunk_to_unary,
    normalize=objectiveai_pyo3.normalize_function_invention_recursive_for_tests,
    cases=[
        HttpTestCase(
            snapshot="valid_schema_valid_tasks_scalar_leaf",
            body={
                "remote": "mock",
                "state": {
                    "type": "alpha.scalar.leaf.function",
                    "depth": 0, "min_branch_width": 1, "max_branch_width": 1,
                    "min_leaf_width": 2, "max_leaf_width": 4,
                    "name": "inv-good-sl",
                    "spec": "Test function spec for mock invention.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "sentiment": {"type": "string", "enum": ["positive", "negative"]},
                        },
                        "required": ["sentiment"],
                    },
                    "essay_tasks": "Good tasks incoming.",
                    "tasks": [
                        {
                            "type": "vector.completion",
                            "messages": {"$starlark": '[{"role": "user", "content": [{"type": "text", "text": str(input)}]}]'},
                            "responses": [[{"type": "text", "text": "yes"}], [{"type": "text", "text": "no"}]],
                        },
                        {
                            "type": "vector.completion",
                            "messages": {"$starlark": '[{"role": "user", "content": [{"type": "text", "text": str(input)}]}]'},
                            "responses": [[{"type": "text", "text": "yes"}], [{"type": "text", "text": "no"}]],
                        },
                    ],
                    "tasks_length": 2,
                    "description": "A valid scalar function.",
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 5300,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
        HttpTestCase(
            snapshot="valid_vector_schema_valid_tasks",
            body={
                "remote": "mock",
                "state": {
                    "type": "alpha.vector.leaf.function",
                    "depth": 0, "min_branch_width": 1, "max_branch_width": 1,
                    "min_leaf_width": 2, "max_leaf_width": 4,
                    "name": "inv-good-vl",
                    "spec": "Test function spec for mock invention.",
                    "essay": "Ranking things.",
                    "input_schema": {
                        "items": {"type": "string", "enum": ["apple", "banana"]},
                    },
                    "tasks": [
                        {
                            "type": "vector.completion",
                            "messages": {"$starlark": '[{"role": "user", "content": [{"type": "text", "text": "rank these"}]}]'},
                            "responses": {"$starlark": '[[{"type": "text", "text": str(item)}] for item in input[\'items\']]'},
                        },
                        {
                            "type": "vector.completion",
                            "messages": {"$starlark": '[{"role": "user", "content": [{"type": "text", "text": "rank these"}]}]'},
                            "responses": {"$starlark": '[[{"type": "text", "text": str(item)}] for item in input[\'items\']]'},
                        },
                    ],
                    "tasks_length": 2,
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 5400,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
        HttpTestCase(
            snapshot="valid_schema_no_tasks_with_essay",
            body={
                "remote": "mock",
                "state": {
                    "type": "alpha.scalar.leaf.function",
                    "depth": 0, "min_branch_width": 1, "max_branch_width": 1,
                    "min_leaf_width": 2, "max_leaf_width": 4,
                    "name": "inv-schema-only",
                    "spec": "Test function spec for mock invention.",
                    "essay": "A great essay about things.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "sentiment": {"type": "string", "enum": ["positive", "negative"]},
                        },
                        "required": ["sentiment"],
                    },
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 5900,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
    ],
))
