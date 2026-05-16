"""Methods for inventions AgentCompletionChunk (flattened AgentCompletionChunk + index)."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.functions.inventions.response.streaming.agent_completion_chunk import (
    AgentCompletionChunk,
)


def _push(self, other: AgentCompletionChunk) -> None:
    push_by_index(self.messages, other.messages)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)
    push_replace(self, "continuation", other.continuation)


AgentCompletionChunk.push = _push
