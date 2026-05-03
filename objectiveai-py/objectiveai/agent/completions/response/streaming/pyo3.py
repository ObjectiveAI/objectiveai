"""PyO3 bindings for agent completion chunk operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_agent_completion_chunk_merged(a, b):
    """Merge two AgentCompletionChunks via push."""
    return objectiveai_pyo3.agent_completion_chunk_merged(a, b)


def pyo3_agent_completion_chunk_normalized(a):
    """Normalize an AgentCompletionChunk by round-tripping through serde."""
    return objectiveai_pyo3.agent_completion_chunk_normalized(a)


def pyo3_agent_completion_chunk_to_unary(a):
    """Convert an accumulated AgentCompletionChunk to an AgentCompletion."""
    return objectiveai_pyo3.agent_completion_chunk_to_unary(a)


def pyo3_normalize_agent_completion_for_tests(a):
    """Normalize an AgentCompletion for test snapshot stability."""
    return objectiveai_pyo3.normalize_agent_completion_for_tests(a)


def pyo3_generate_agent_completion_chunk(seed=None):
    """Generate a random AgentCompletionChunk."""
    return objectiveai_pyo3.generate_agent_completion_chunk(seed)
