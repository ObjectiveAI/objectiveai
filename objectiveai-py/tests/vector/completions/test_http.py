"""HTTP integration tests for vector completions."""

import objectiveai._pyo3 as objectiveai_pyo3

from objectiveai.vector.completions.http import create_vector_completion
from objectiveai.vector.completions.response.streaming import VectorCompletionChunk
from tests.http_test_util import HttpTestCase, http_test_suite, ASSETS_DIR

MOCK_AGENT = {"upstream": "mock", "output_mode": "instruction"}

globals().update(http_test_suite(
    name="vector completions http",
    fn=create_vector_completion,
    snapshots_dir=ASSETS_DIR / "vector" / "completions" / "client_tests",
    chunk_cls=VectorCompletionChunk,
    chunk_to_unary=objectiveai_pyo3.vector_completion_chunk_to_unary,
    normalize=objectiveai_pyo3.normalize_vector_completion_for_tests,
    cases=[
        HttpTestCase(
            snapshot="single_agent_2_responses_instruction_seed_42",
            body={
                "messages": [{"role": "user", "content": "Which is better?"}],
                "swarm": {"agents": [MOCK_AGENT]},
                "responses": ["Response A", "Response B"],
                "seed": 42,
            },
        ),
        HttpTestCase(
            snapshot="many_responses_deep_prefix_tree_seed_42",
            body={
                "messages": [{"role": "user", "content": "Pick the best"}],
                "swarm": {"agents": [MOCK_AGENT]},
                "responses": [f"Response {i}" for i in range(25)],
                "seed": 42,
            },
        ),
        HttpTestCase(
            snapshot="mixed_output_modes_seed_88",
            body={
                "messages": [
                    {"role": "user", "content": "Compare these vacation destinations"},
                ],
                "swarm": {
                    "agents": [
                        {"upstream": "mock", "output_mode": "instruction"},
                        {"upstream": "mock", "output_mode": "json_schema"},
                        {"upstream": "mock", "output_mode": "tool_call"},
                    ],
                    "weights": [0.4, 0.3, 0.3],
                },
                "responses": [
                    "Kyoto, Japan",
                    "Reykjavik, Iceland",
                    "Patagonia, Argentina",
                ],
                "seed": 88,
            },
        ),
    ],
))
