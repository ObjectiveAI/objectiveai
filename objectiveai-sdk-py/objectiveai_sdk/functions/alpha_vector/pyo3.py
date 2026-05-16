"""PyO3 bindings for alpha vector function validation."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3


def pyo3_alpha_check_leaf_vector_function(function):
    """Alpha check for a leaf vector function (depth 0, vector output)."""
    return objectiveai_sdk_pyo3.alpha_check_leaf_vector_function(function)


def pyo3_alpha_check_branch_vector_function(function, children=None):
    """Alpha check for a branch vector function (depth > 0, vector output)."""
    return objectiveai_sdk_pyo3.alpha_check_branch_vector_function(function, children)
