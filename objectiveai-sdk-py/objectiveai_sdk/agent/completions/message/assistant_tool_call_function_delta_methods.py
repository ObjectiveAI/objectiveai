"""Methods for AssistantToolCallFunctionDelta."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_option_string
from objectiveai_sdk.agent.completions.message.assistant_tool_call_function_delta import (
    AssistantToolCallFunctionDelta,
)


def _push(self, other: AssistantToolCallFunctionDelta) -> None:
    if self.name is None and other.name is not None:
        self.name = other.name
    push_option_string(self, "arguments", other.arguments)


AssistantToolCallFunctionDelta.push = _push
