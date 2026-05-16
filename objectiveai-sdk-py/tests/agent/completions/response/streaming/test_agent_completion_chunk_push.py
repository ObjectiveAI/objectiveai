"""Fuzz test: Python push vs PyO3 (Rust) push for AgentCompletionChunk."""
import copy

import pytest

objectiveai_sdk_pyo3 = pytest.importorskip("objectiveai._pyo3")

from objectiveai_sdk.agent.completions.response.streaming import AgentCompletionChunk
from tests.push_test_utils import rounded


@pytest.mark.parametrize("stream", range(20))
def test_push_fuzz(stream):
    seed = stream * 1000
    init = objectiveai_sdk_pyo3.generate_agent_completion_chunk(seed)
    py_acc = AgentCompletionChunk.model_validate(init)
    pyo3_acc = copy.deepcopy(init)
    seed += 1

    for j in range(20):
        chunk = objectiveai_sdk_pyo3.generate_agent_completion_chunk(seed)
        seed += 1

        py_acc.push(AgentCompletionChunk.model_validate(chunk))
        pyo3_acc = objectiveai_sdk_pyo3.agent_completion_chunk_merged(pyo3_acc, chunk)

        py_dict = py_acc.model_dump(mode="python", by_alias=True, exclude_unset=True)
        assert rounded(py_dict) == rounded(pyo3_acc), f"chunk {j}"
