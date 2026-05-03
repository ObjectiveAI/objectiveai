"""Errors raised by objectiveai-cocoindex wrappers."""

from __future__ import annotations


class ObjectiveAIExecutionError(RuntimeError):
    """Raised when a function execution returns an error output (TaskOutputErr)."""

    def __init__(self, payload: object) -> None:
        super().__init__(f"ObjectiveAI execution error: {payload!r}")
        self.payload = payload
