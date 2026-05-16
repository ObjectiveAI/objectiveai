"""Methods for TaskChunk (Union dispatch)."""
from __future__ import annotations

from objectiveai_sdk.functions.executions.response.streaming.task_chunk import (
    TaskChunk,
)
from objectiveai_sdk.functions.executions.response.streaming.function_execution_task_chunk import (
    FunctionExecutionTaskChunk,
)
from objectiveai_sdk.functions.executions.response.streaming.vector_completion_task_chunk import (
    VectorCompletionTaskChunk,
)


def _push(self, other: TaskChunk) -> None:
    a = self.root
    b = other.root
    if type(a) is type(b):
        # Unwrap RootModel variants to get the inner type with push()
        a.root.push(b.root)


TaskChunk.push = _push
