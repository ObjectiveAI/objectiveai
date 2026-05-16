"""Methods for UpstreamUsage."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_option
from objectiveai_sdk.agent.completions.response.upstream_usage import (
    UpstreamUsage,
)


def _push(self, other: UpstreamUsage) -> None:
    self.completion_tokens += other.completion_tokens
    self.prompt_tokens += other.prompt_tokens
    self.total_tokens += other.total_tokens
    self.cost += other.cost
    self.total_cost += other.total_cost
    push_option(self, "completion_tokens_details", other.completion_tokens_details)
    push_option(self, "prompt_tokens_details", other.prompt_tokens_details)
    push_option(self, "cost_details", other.cost_details)


UpstreamUsage.push = _push
