"""Methods for AssistantResponseChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_option_string, push_replace
from objectiveai_sdk.agent.completions.response.streaming.assistant_response_chunk import (
    AssistantResponseChunk,
)


def _push(self, other: AssistantResponseChunk) -> None:
    push_option_string(self, "reasoning", other.reasoning)

    if self.tool_calls is not None and other.tool_calls is not None:
        push_by_index(self.tool_calls, other.tool_calls)
    elif other.tool_calls is not None:
        self.tool_calls = list(other.tool_calls)

    push_option(self, "content", other.content)
    push_option_string(self, "refusal", other.refusal)

    # finish_reason: lazy set (first wins)
    if self.finish_reason is None and other.finish_reason is not None:
        self.finish_reason = other.finish_reason

    push_option(self, "logprobs", other.logprobs)

    if not self.upstream_id and other.upstream_id:
        self.upstream_id = other.upstream_id

    if self.service_tier is None and other.service_tier is not None:
        self.service_tier = other.service_tier

    if self.system_fingerprint is None and other.system_fingerprint is not None:
        self.system_fingerprint = other.system_fingerprint

    if self.provider is None and other.provider is not None:
        self.provider = other.provider

    push_option(self, "usage", other.usage)


AssistantResponseChunk.push = _push
