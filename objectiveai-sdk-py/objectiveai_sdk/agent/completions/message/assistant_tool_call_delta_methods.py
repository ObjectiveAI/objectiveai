"""Methods for AssistantToolCallDelta."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_option
from objectiveai_sdk.agent.completions.message.assistant_tool_call_delta import (
    AssistantToolCallDelta,
)


def _push(self, other: AssistantToolCallDelta) -> None:
    if self.type_ is None and other.type_ is not None:
        self.type_ = other.type_
    if self.id is None and other.id is not None:
        self.id = other.id
    push_option(self, "function", other.function)


AssistantToolCallDelta.push = _push
