"""Methods for FunctionInventionRecursiveChunk."""
from __future__ import annotations

from objectiveai_sdk.push_utils import push_by_index, push_option, push_lazy_set_true
from objectiveai_sdk.functions.inventions.recursive.response.streaming.function_invention_recursive_chunk import (
    FunctionInventionRecursiveChunk,
)


def _push(self, other: FunctionInventionRecursiveChunk) -> None:
    push_by_index(self.inventions, other.inventions)
    push_lazy_set_true(self, "inventions_errors", other.inventions_errors)
    push_option(self, "usage", other.usage)


FunctionInventionRecursiveChunk.push = _push
