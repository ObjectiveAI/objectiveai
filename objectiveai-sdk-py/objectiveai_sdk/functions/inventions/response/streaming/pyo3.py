"""PyO3 bindings for function invention chunk operations."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3


def pyo3_function_invention_chunk_merged(a, b):
    """Merge two FunctionInventionChunks via push."""
    return objectiveai_sdk_pyo3.function_invention_chunk_merged(a, b)


def pyo3_function_invention_chunk_normalized(a):
    """Normalize a FunctionInventionChunk by round-tripping through serde."""
    return objectiveai_sdk_pyo3.function_invention_chunk_normalized(a)


def pyo3_function_invention_chunk_to_unary(a):
    """Convert an accumulated FunctionInventionChunk to a FunctionInvention."""
    return objectiveai_sdk_pyo3.function_invention_chunk_to_unary(a)


def pyo3_normalize_function_invention_for_tests(a):
    """Normalize a FunctionInvention for test snapshot stability."""
    return objectiveai_sdk_pyo3.normalize_function_invention_for_tests(a)


def pyo3_generate_function_invention_chunk(seed=None):
    """Generate a random FunctionInventionChunk."""
    return objectiveai_sdk_pyo3.generate_function_invention_chunk(seed)
