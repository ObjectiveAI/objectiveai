"""Methods for recursive FunctionInventionChunk (flattened FunctionInventionChunk + index)."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_replace
from objectiveai_sdk.functions.inventions.recursive.response.streaming.function_invention_chunk import (
    FunctionInventionChunk,
)


def _push(self, other: FunctionInventionChunk) -> None:
    push_by_index(self.completions, other.completions)
    push_replace(self, "state", other.state)
    push_replace(self, "path", other.path)
    push_replace(self, "function", other.function)
    push_replace(self, "error", other.error)
    push_option(self, "usage", other.usage)


FunctionInventionChunk.push = _push
