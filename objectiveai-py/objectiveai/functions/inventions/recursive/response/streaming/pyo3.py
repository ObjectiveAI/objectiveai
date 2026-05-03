"""PyO3 bindings for recursive function invention chunk operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_function_invention_recursive_chunk_merged(a, b):
    """Merge two FunctionInventionRecursiveChunks via push."""
    return objectiveai_pyo3.function_invention_recursive_chunk_merged(a, b)


def pyo3_function_invention_recursive_chunk_normalized(a):
    """Normalize a FunctionInventionRecursiveChunk by round-tripping through serde."""
    return objectiveai_pyo3.function_invention_recursive_chunk_normalized(a)


def pyo3_function_invention_recursive_chunk_to_unary(a):
    """Convert an accumulated FunctionInventionRecursiveChunk to a FunctionInventionRecursive."""
    return objectiveai_pyo3.function_invention_recursive_chunk_to_unary(a)


def pyo3_normalize_function_invention_recursive_for_tests(a):
    """Normalize a FunctionInventionRecursive for test snapshot stability."""
    return objectiveai_pyo3.normalize_function_invention_recursive_for_tests(a)


def pyo3_generate_function_invention_recursive_chunk(seed=None):
    """Generate a random FunctionInventionRecursiveChunk."""
    return objectiveai_pyo3.generate_function_invention_recursive_chunk(seed)
