"""Methods for VectorCompletionTaskChunk (flattened VectorCompletionChunk + index fields)."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.functions.executions.response.streaming.vector_completion_task_chunk import (
    VectorCompletionTaskChunk,
)


def _push(self, other: VectorCompletionTaskChunk) -> None:
    push_by_index(self.completions, other.completions)
    self.votes.extend(other.votes)
    self.scores = list(other.scores)
    self.weights = list(other.weights)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)
    # request_messages / request_choices: first chunk wins (ride only
    # the task's first chunk; never overwritten)
    if self.request_messages is None and other.request_messages is not None:
        self.request_messages = other.request_messages
    if self.request_choices is None and other.request_choices is not None:
        self.request_choices = other.request_choices


VectorCompletionTaskChunk.push = _push
