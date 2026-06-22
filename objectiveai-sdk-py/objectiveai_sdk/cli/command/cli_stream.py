"""CliStream — async-iterable over a command executor's JSONL line stream.

Port of ``objectiveai-sdk-js/src/cli/command/cliStream.ts``. Used by the
generated per-command execute functions; plugin authors normally receive one
rather than constructing it.

Each raw line is validated with a pydantic ``TypeAdapter`` — typically
``Union[CliError, <Response>]`` (left-to-right), so error envelopes and response
values surface as plain union members rather than raised exceptions.

Before validation, externally-tagged aggregate layers are unwrapped: the cli
prints the aggregate ``ResponseItem`` wire shape (e.g. ``{"Config": {"Viewer":
{"Get": {}}}}``), so single-key object wrappers are peeled until the adapter
accepts — the Python mirror of ``extract_leaf`` in
``objectiveai-cli/src/executor.rs``.

The host's synthetic ``{"type": "end"}`` terminator line is consumed (it ends
iteration) and never yielded.
"""
from __future__ import annotations

from typing import Any, AsyncIterable, AsyncIterator, Generic, List, Optional, TypeVar

from pydantic import TypeAdapter

T = TypeVar("T")


class CliStream(Generic[T]):
    def __init__(self, source: AsyncIterable[Any], adapter: TypeAdapter) -> None:
        self._source = source
        self._adapter = adapter

    async def __aiter__(self) -> AsyncIterator[T]:
        async for line in self._source:
            if _is_end_marker(line):
                # The underlying iterable terminates itself after the end
                # marker; skipping here just keeps it out of the typed stream.
                continue
            yield _validate_unwrapping(self._adapter, line)

    async def to_list(self) -> List[T]:
        """Collect every remaining item."""
        items: List[T] = []
        async for item in self:
            items.append(item)
        return items

    async def first(self) -> Optional[T]:
        """Resolve the first item and discard the rest of the stream. ``None``
        when the stream ends without yielding (a unary command that printed
        nothing before the end marker)."""
        agen = self.__aiter__()
        try:
            async for item in agen:
                return item
            return None
        finally:
            await agen.aclose()


def _is_end_marker(line: Any) -> bool:
    """The host's synthetic ``{"type": "end"}`` stream terminator."""
    return isinstance(line, dict) and line.get("type") == "end"


def _validate_unwrapping(adapter: TypeAdapter, value: Any) -> Any:
    """Validate ``value`` with ``adapter``, peeling externally-tagged single-key
    object layers (``{"Agents": {"Spawn": ...}}``) until it accepts — the Python
    mirror of the cli's ``extract_leaf``. On total failure, re-validate the
    ORIGINAL value so the error shows the full wire shape."""
    current = value
    while True:
        try:
            return adapter.validate_python(current)
        except Exception:
            if isinstance(current, dict) and len(current) == 1:
                current = next(iter(current.values()))
                continue
            return adapter.validate_python(value)
