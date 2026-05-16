"""Methods for EvaluationChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.laboratories.executions.response.streaming.evaluation_chunk import (
    EvaluationChunk,
)


def _push(self, other: EvaluationChunk) -> None:
    push_by_index(self.messages, other.messages)
    push_option(self, "usage", other.usage)
    push_replace(self, "error", other.error)
    push_replace(self, "continuation", other.continuation)
    push_replace(self, "output", other.output)


EvaluationChunk.push = _push
