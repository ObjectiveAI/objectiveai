"""PyO3 bindings for message ID computation."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_prompt_id(prompt):
    """Compute a content-addressed ID for chat messages."""
    return objectiveai_pyo3.prompt_id(prompt)


def pyo3_vector_response_id(response):
    """Compute a content-addressed ID for a vector completion response option."""
    return objectiveai_pyo3.vector_response_id(response)
