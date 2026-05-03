"""HTTP integration tests for function inventions."""

import objectiveai._pyo3 as objectiveai_pyo3

from objectiveai.functions.inventions.http import create_function_invention
from objectiveai.functions.inventions.response.streaming import FunctionInventionChunk
from tests.http_test_util import HttpTestCase, http_test_suite, ASSETS_DIR

MOCK_INVENTION_AGENT = {"upstream": "mock", "output_mode": "instruction", "mode": "invention"}

globals().update(http_test_suite(
    name="function inventions http",
    fn=create_function_invention,
    snapshots_dir=ASSETS_DIR / "functions" / "inventions" / "client_tests",
    chunk_cls=FunctionInventionChunk,
    chunk_to_unary=objectiveai_pyo3.function_invention_chunk_to_unary,
    normalize=objectiveai_pyo3.normalize_function_invention_for_tests,
    cases=[
        HttpTestCase(
            snapshot="scalar_leaf_s42_0",
            body={
                "state": {
                    "type": "alpha.scalar.leaf.function",
                    "depth": 0, "min_branch_width": 3, "max_branch_width": 5,
                    "min_leaf_width": 3, "max_leaf_width": 5,
                    "name": "sl-default",
                    "spec": "Test function spec for mock invention.",
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 42,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
        HttpTestCase(
            snapshot="vector_branch_s2025_0",
            body={
                "state": {
                    "type": "alpha.vector.branch.function",
                    "depth": 3, "min_branch_width": 2, "max_branch_width": 4,
                    "min_leaf_width": 2, "max_leaf_width": 4,
                    "name": "vb-deep",
                    "spec": "Test function spec for mock invention.",
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 2025,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
        HttpTestCase(
            snapshot="scalar_leaf_schema_kitchen_0",
            body={
                "state": {
                    "type": "alpha.scalar.leaf.function",
                    "depth": 0, "min_branch_width": 3, "max_branch_width": 5,
                    "min_leaf_width": 3, "max_leaf_width": 5,
                    "name": "sl-kitchen",
                    "spec": "Test function spec for mock invention.",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "age": {"type": "integer"},
                            "score": {"type": "number"},
                            "active": {"type": "boolean"},
                            "avatar": {"type": "image"},
                            "voicemail": {"type": "audio"},
                            "demo": {"type": "video"},
                            "resume": {"type": "file"},
                            "aliases": {
                                "type": "array",
                                "items": {"anyOf": [{"type": "string"}, {"type": "integer"}]},
                                "minItems": 1,
                                "maxItems": 8,
                            },
                            "extra": {
                                "anyOf": [
                                    {"type": "string"},
                                    {
                                        "type": "array",
                                        "items": {
                                            "type": "object",
                                            "properties": {
                                                "key": {"type": "string"},
                                                "val": {"anyOf": [{"type": "number"}, {"type": "boolean"}, {"type": "image"}]},
                                            },
                                            "required": ["key", "val"],
                                        },
                                        "minItems": 1,
                                        "maxItems": 3,
                                    },
                                ],
                            },
                        },
                        "required": ["name", "age", "score", "active", "avatar", "voicemail", "demo", "resume", "aliases", "extra"],
                    },
                },
                "agent": MOCK_INVENTION_AGENT,
                "prompt": {"remote": "mock", "name": "default"},
                "seed": 80004,
                "stream": True,
                "max_step_retries": 1,
            },
        ),
    ],
))
