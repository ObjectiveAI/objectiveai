"""Methods for LaboratoryExecutionChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.laboratories.executions.response.streaming.laboratory_execution_chunk import (
    LaboratoryExecutionChunk,
)


def _push(self, other: LaboratoryExecutionChunk) -> None:
    push_by_index(self.builders, other.builders)
    push_by_index(self.evaluations, other.evaluations)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)


LaboratoryExecutionChunk.push = _push
