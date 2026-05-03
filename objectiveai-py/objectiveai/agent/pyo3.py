"""PyO3 bindings for agent operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_validate_agent(agent):
    """Validate an agent configuration and compute its content-addressed ID."""
    return objectiveai_pyo3.validate_agent(agent)
