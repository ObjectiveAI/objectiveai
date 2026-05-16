"""Methods for FunctionProfileComputationChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace, push_lazy_set_true
from objectiveai_sdk.functions.profiles.computations.response.streaming.function_profile_computation_chunk import (
    FunctionProfileComputationChunk,
)


def _push(self, other: FunctionProfileComputationChunk) -> None:
    push_by_index(self.executions, other.executions)
    push_lazy_set_true(self, "executions_errors", other.executions_errors)
    push_replace(self, "profile", other.profile)
    push_replace(self, "fitting_stats", other.fitting_stats)
    push_replace(self, "retry_token", other.retry_token)
    push_option(self, "usage", other.usage)


FunctionProfileComputationChunk.push = _push
