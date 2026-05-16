"""Methods for VectorCompletionChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option
from objectiveai_sdk.vector.completions.response.streaming.vector_completion_chunk import (
    VectorCompletionChunk,
)


def _push(self, other: VectorCompletionChunk) -> None:
    push_by_index(self.completions, other.completions)
    self.votes.extend(other.votes)
    self.scores = list(other.scores)
    self.weights = list(other.weights)
    push_option(self, "usage", other.usage)


VectorCompletionChunk.push = _push
