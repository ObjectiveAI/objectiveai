"""PyO3 bindings for function field validation."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_check_vector_fields(fields):
    """Validate vector function fields (output_length, input_split, input_merge)."""
    return objectiveai_pyo3.check_vector_fields(fields)


def pyo3_check_scalar_fields(fields):
    """Validate scalar function fields (input_schema only)."""
    return objectiveai_pyo3.check_scalar_fields(fields)
