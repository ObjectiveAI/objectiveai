"""PyO3 bindings for swarm operations."""

import objectiveai._pyo3 as objectiveai_pyo3


def pyo3_validate_swarm(swarm, remote_agents=None):
    """Validate an swarm configuration and compute its content-addressed ID."""
    return objectiveai_pyo3.validate_swarm(swarm, remote_agents)
