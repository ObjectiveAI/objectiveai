"""Methods for ReasoningSummaryChunk (flattened AgentCompletionChunk + error)."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.functions.executions.response.streaming.reasoning_summary_chunk import (
    ReasoningSummaryChunk,
)


def _push(self, other: ReasoningSummaryChunk) -> None:
    push_by_index(self.messages, other.messages)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)
    push_replace(self, "continuation", other.continuation)


ReasoningSummaryChunk.push = _push
