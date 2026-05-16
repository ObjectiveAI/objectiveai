"""Methods for CostDetails."""
from __future__ import annotations

from objectiveai_sdk.agent.completions.response.cost_details import (
    CostDetails,
)


def _push(self, other: CostDetails) -> None:
    self.upstream_inference_cost += other.upstream_inference_cost
    self.upstream_upstream_inference_cost += other.upstream_upstream_inference_cost


CostDetails.push = _push
