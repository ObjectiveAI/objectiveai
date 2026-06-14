"""Methods for FunctionExecutionTaskChunk (flattened FunctionExecutionChunk + index fields)."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace, push_lazy_set_true
from objectiveai_sdk.functions.executions.response.streaming.function_execution_task_chunk import (
    FunctionExecutionTaskChunk,
)


def _push(self, other: FunctionExecutionTaskChunk) -> None:
    push_by_index(self.tasks, other.tasks)
    push_lazy_set_true(self, "tasks_errors", other.tasks_errors)
    push_option(self, "reasoning", other.reasoning)
    push_replace(self, "output", other.output)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)


FunctionExecutionTaskChunk.push = _push
