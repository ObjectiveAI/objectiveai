"""HTTP integration tests for agent completions."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3

from objectiveai_sdk.agent.completions.http import create_agent_completion
from objectiveai_sdk.agent.completions.response.streaming import AgentCompletionChunk
from tests.http_test_util import HttpTestCase, http_test_suite, ASSETS_DIR

globals().update(http_test_suite(
    name="agent completions http",
    fn=create_agent_completion,
    snapshots_dir=ASSETS_DIR / "agent" / "completions" / "client_tests",
    chunk_cls=AgentCompletionChunk,
    chunk_to_unary=objectiveai_sdk_pyo3.agent_completion_chunk_to_unary,
    normalize=objectiveai_sdk_pyo3.normalize_agent_completion_for_tests,
    cases=[
        HttpTestCase(
            snapshot="test_basic_mock_agent_seed_42",
            body={
                "messages": [],
                "agent": {"upstream": "mock", "output_mode": "instruction"},
                "seed": 42,
            },
        ),
        HttpTestCase(
            snapshot="test_with_developer_and_user_messages",
            body={
                "messages": [
                    {"role": "developer", "content": "You are a helpful assistant."},
                    {"role": "user", "content": "What is 2+2?"},
                ],
                "agent": {"upstream": "mock", "output_mode": "instruction"},
                "seed": 99,
            },
        ),
        HttpTestCase(
            snapshot="test_json_object_response_format",
            body={
                "messages": [],
                "agent": {"upstream": "mock", "output_mode": "instruction"},
                "response_format": {"type": "json_object"},
                "seed": 42,
            },
        ),
    ],
))
