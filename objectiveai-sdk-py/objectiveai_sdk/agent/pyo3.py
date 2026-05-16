"""PyO3 bindings for agent operations."""

import objectiveai_sdk._pyo3 as objectiveai_sdk_pyo3


def pyo3_validate_agent(agent):
    """Validate an agent configuration and compute its content-addressed ID."""
    return objectiveai_sdk_pyo3.validate_agent(agent)
