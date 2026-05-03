"""ObjectiveAI integration for `cocoindex <https://github.com/cocoindex-io/cocoindex>`_.

Exposes a single ``Function`` class that runs ObjectiveAI function executions
as memoized cocoindex processing components. Memo key combines the bound
``(function, profile, strategy)`` triple with the per-call ``input``.

The ``client`` is intentionally excluded from the memo key — two
``Function`` instances over the same triple with different clients share
cache entries. This makes the library safe to drop into pipelines without
worrying about client identity.
"""

from __future__ import annotations

from objectiveai_cocoindex._client import set_default_client
from objectiveai_cocoindex._errors import ObjectiveAIExecutionError
from objectiveai_cocoindex._function import Function
from objectiveai_cocoindex._sources import (
    FunctionSource,
    InlineFunction,
    InlineProfile,
    ProfileSource,
    RemoteFunction,
    RemoteProfile,
)

__all__ = [
    "Function",
    "set_default_client",
    "FunctionSource",
    "ProfileSource",
    "RemoteFunction",
    "InlineFunction",
    "RemoteProfile",
    "InlineProfile",
    "ObjectiveAIExecutionError",
]
