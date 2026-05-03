"""PyO3 bindings for function profile computation chunk operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_function_profile_computation_chunk_merged(a, b):
    """Merge two FunctionProfileComputationChunks via push."""
    return objectiveai_pyo3.function_profile_computation_chunk_merged(a, b)


def pyo3_function_profile_computation_chunk_normalized(a):
    """Normalize a FunctionProfileComputationChunk by round-tripping through serde."""
    return objectiveai_pyo3.function_profile_computation_chunk_normalized(a)


def pyo3_function_profile_computation_chunk_to_unary(a):
    """Convert an accumulated FunctionProfileComputationChunk to a FunctionProfileComputation."""
    return objectiveai_pyo3.function_profile_computation_chunk_to_unary(a)


def pyo3_normalize_function_profile_computation_for_tests(a):
    """Normalize a FunctionProfileComputation for test snapshot stability."""
    return objectiveai_pyo3.normalize_function_profile_computation_for_tests(a)


def pyo3_generate_function_profile_computation_chunk(seed=None):
    """Generate a random FunctionProfileComputationChunk."""
    return objectiveai_pyo3.generate_function_profile_computation_chunk(seed)
