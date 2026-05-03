"""PyO3 bindings for LaboratoryExecutionChunk operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_laboratory_execution_chunk_merged(a, b):
    """Merge two LaboratoryExecutionChunks via push."""
    return objectiveai_pyo3.laboratory_execution_chunk_merged(a, b)


def pyo3_laboratory_execution_chunk_normalized(a):
    """Normalize a LaboratoryExecutionChunk by round-tripping through serde."""
    return objectiveai_pyo3.laboratory_execution_chunk_normalized(a)


def pyo3_laboratory_execution_chunk_to_unary(a):
    """Convert a LaboratoryExecutionChunk to unary."""
    return objectiveai_pyo3.laboratory_execution_chunk_to_unary(a)


def pyo3_normalize_laboratory_execution_for_tests(a):
    """Normalize a LaboratoryExecution for test snapshot stability."""
    return objectiveai_pyo3.normalize_laboratory_execution_for_tests(a)


def pyo3_generate_laboratory_execution_chunk(seed=None):
    """Generate a random LaboratoryExecutionChunk."""
    return objectiveai_pyo3.generate_laboratory_execution_chunk(seed)
