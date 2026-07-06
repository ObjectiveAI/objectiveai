"""Methods for AgentCompletionChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.agent.completions.response.streaming.agent_completion_chunk import (
    AgentCompletionChunk,
)


def _push(self, other: AgentCompletionChunk) -> None:
    push_by_index(self.messages, other.messages)
    push_option(self, "usage", other.usage)
    push_replace(self, "error", other.error)
    push_replace(self, "continuation", other.continuation)
    # agent_inline: first chunk wins (rides only the completion's
    # first chunk; never overwritten)
    if self.agent_inline is None and other.agent_inline is not None:
        self.agent_inline = other.agent_inline


AgentCompletionChunk.push = _push
