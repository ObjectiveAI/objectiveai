"""PyO3 bindings for function execution chunk operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_function_execution_chunk_merged(a, b):
    """Merge two FunctionExecutionChunks via push."""
    return objectiveai_pyo3.function_execution_chunk_merged(a, b)


def pyo3_function_execution_chunk_normalized(a):
    """Normalize a FunctionExecutionChunk by round-tripping through serde."""
    return objectiveai_pyo3.function_execution_chunk_normalized(a)


def pyo3_function_execution_chunk_to_unary(a):
    """Convert an accumulated FunctionExecutionChunk to a FunctionExecution."""
    return objectiveai_pyo3.function_execution_chunk_to_unary(a)


def pyo3_normalize_function_execution_for_tests(a):
    """Normalize a FunctionExecution for test snapshot stability."""
    return objectiveai_pyo3.normalize_function_execution_for_tests(a)


def pyo3_generate_function_execution_chunk(seed=None):
    """Generate a random FunctionExecutionChunk."""
    return objectiveai_pyo3.generate_function_execution_chunk(seed)
