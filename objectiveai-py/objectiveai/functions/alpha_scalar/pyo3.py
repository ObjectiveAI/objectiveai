"""PyO3 bindings for alpha scalar function validation."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_alpha_check_leaf_scalar_function(function):
    """Alpha check for a leaf scalar function (depth 0, scalar output)."""
    return objectiveai_pyo3.alpha_check_leaf_scalar_function(function)


def pyo3_alpha_check_branch_scalar_function(function, children=None):
    """Alpha check for a branch scalar function (depth > 0, scalar output)."""
    return objectiveai_pyo3.alpha_check_branch_scalar_function(function, children)
