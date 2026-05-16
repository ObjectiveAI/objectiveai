"""Server-Sent Events (SSE) stream for ObjectiveAI API responses."""

from __future__ import annotations

import json
from typing import Any, AsyncIterator, Generic, TypeVar

from objectiveai_sdk.error import ObjectiveAIFetchError, _is_response_error

T = TypeVar("T")


class Stream(Generic[T]):
    """Async iterator over Server-Sent Events from an API response.

    Parses SSE-formatted data from an ``httpx`` streaming response and
    yields parsed JSON chunks.  Raises :class:`ObjectiveAIFetchError` if
    an error event is received in the stream.

    Usage::

        async with client.post_streaming("/path", body) as stream:
            async for chunk in stream:
                print(chunk)
    """

    def __init__(self, response: Any) -> None:  # httpx.Response
        self._response = response
        self._buffer = ""
        self._done = False

    async def __aiter__(self) -> AsyncIterator[T]:
        try:
            async for raw_bytes in self._response.aiter_bytes():
                if self._done:
                    break

                self._buffer += raw_bytes.decode("utf-8", errors="replace")

                # Split by double newline (SSE event separator)
                parts = self._buffer.split("\n\n")

                # Keep the last part in the buffer (might be incomplete)
                self._buffer = parts.pop()

                for part in parts:
                    for event in self._parse_sse(part):
                        yield event
                        if self._done:
                            return

            # Process any remaining data in buffer
            if self._buffer.strip():
                for event in self._parse_sse(self._buffer):
                    yield event
        finally:
            await self._response.aclose()

    def _parse_sse(self, text: str) -> list[T]:
        """Parse an SSE event block and return parsed data events."""
        results: list[T] = []
        for line in text.split("\n"):
            if not line or line.startswith(":"):
                continue

            if line.startswith("data:"):
                data = line[5:].strip()

                if data == "[DONE]":
                    self._done = True
                    continue

                if not data:
                    continue

                parsed = json.loads(data)

                if _is_response_error(parsed):
                    raise ObjectiveAIFetchError(parsed)

                results.append(parsed)

        return results

    async def to_list(self) -> list[T]:
        """Collect all events into a list."""
        items: list[T] = []
        async for item in self:
            items.append(item)
        return items
