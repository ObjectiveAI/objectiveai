"""ObjectiveAI API error types."""

from __future__ import annotations

import json
from typing import Any


class ObjectiveAIFetchError(Exception):
    """Error raised when an API request fails.

    Attributes:
        body: The complete error response (contains code and message).
        code: The HTTP status code.
    """

    body: dict[str, Any]

    def __init__(
        self,
        code_or_body: int | dict[str, Any],
        raw_body: str | None = None,
    ) -> None:
        if isinstance(code_or_body, dict):
            # Direct ResponseError dict (e.g. from streaming)
            body = code_or_body
        elif raw_body is None:
            body = {"code": code_or_body, "message": None}
        else:
            try:
                parsed = json.loads(raw_body)
            except (json.JSONDecodeError, ValueError):
                body = {"code": code_or_body, "message": raw_body}
                super().__init__(json.dumps(body))
                self.body = body
                return

            if _is_response_error(parsed):
                body = parsed
            else:
                body = {"code": code_or_body, "message": parsed}

        super().__init__(json.dumps(body))
        self.body = body

    @property
    def code(self) -> int:
        """The HTTP status code."""
        return self.body["code"]


def _is_response_error(obj: Any) -> bool:
    """Check if an object looks like a ResponseError."""
    return (
        isinstance(obj, dict)
        and "code" in obj
        and isinstance(obj["code"], int)
        and "message" in obj
    )
