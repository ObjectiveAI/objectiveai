"""PyO3 bindings for vector completion chunk operations."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3


def pyo3_vector_completion_chunk_merged(a, b):
    """Merge two VectorCompletionChunks via push."""
    return objectiveai_sdk_pyo3.vector_completion_chunk_merged(a, b)


def pyo3_vector_completion_chunk_normalized(a):
    """Normalize a VectorCompletionChunk by round-tripping through serde."""
    return objectiveai_sdk_pyo3.vector_completion_chunk_normalized(a)


def pyo3_vector_completion_chunk_to_unary(a):
    """Convert an accumulated VectorCompletionChunk to a VectorCompletion."""
    return objectiveai_sdk_pyo3.vector_completion_chunk_to_unary(a)


def pyo3_normalize_vector_completion_for_tests(a):
    """Normalize a VectorCompletion for test snapshot stability."""
    return objectiveai_sdk_pyo3.normalize_vector_completion_for_tests(a)


def pyo3_generate_vector_completion_chunk(seed=None):
    """Generate a random VectorCompletionChunk."""
    return objectiveai_sdk_pyo3.generate_vector_completion_chunk(seed)
