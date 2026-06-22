"""In-process plugin command executor.

A plugin authored in Python uses this to ask the HOST to run CLI commands over
the process's own stdin/stdout, speaking the NDJSON command protocol. Direct
port of ``objectiveai-sdk-js/src/cli/command/executor/plugin.ts`` (itself a port
of the Rust ``PluginCommandExecutor``). Async-only.

It reproduces the four primitives that make PARALLEL callers safe (the whole
point — many concurrent ``execute()`` calls must share one stdin/stdout without
corrupting it):

1. ONE reader. A single background thread owns ``sys.stdin``; no caller reads
   stdin directly, so concurrent calls never race on the read. (asyncio can't
   portably wrap ``sys.stdin`` — especially on Windows — so a blocking reader
   thread hands each line to the loop via ``loop.call_soon_threadsafe``.)
2. id -> queue demux. Each call mints a monotonic id and registers an
   ``asyncio.Queue``; the reader routes each line to the matching id's queue.
3. Serialized writes. All writes to stdout go through one ``asyncio.Lock`` so
   two concurrent callers can't interleave partial NDJSON lines.
4. Liveness recheck. On stdin EOF the reader sets ``alive = False`` and ends
   every pending queue; each ``execute()`` inserts THEN rechecks ``alive``, so a
   call racing the close can't strand a registered queue.

As in Rust/JS there is exactly one instance per process (it captures the global
stdin/stdout): use :meth:`PluginCommandExecutor.instance`.
"""
from __future__ import annotations

import asyncio
import json
import sys
import threading
from typing import Any, AsyncIterator, Dict, Optional

# Queue sentinel: this call's stream is complete.
_END = object()


class PluginCommandExecutor:
    _instance: Optional["PluginCommandExecutor"] = None

    def __init__(self) -> None:
        self._counter = 0
        self._pending: Dict[str, asyncio.Queue] = {}
        self._alive = True
        self._loop: Optional[asyncio.AbstractEventLoop] = None
        self._write_lock = asyncio.Lock()
        self._reader_started = False

    @classmethod
    def instance(cls) -> "PluginCommandExecutor":
        """The process-wide singleton (captures the global stdin/stdout)."""
        if cls._instance is None:
            cls._instance = cls()
        return cls._instance

    async def execute(self, request: Any) -> AsyncIterator[Any]:
        """Send ``request`` to the host and stream back its responses (yielded
        raw; the caller's :class:`CliStream` discriminates errors vs responses).
        Safe to call concurrently from any number of parallel callers.
        """
        self._ensure_reader()

        ident = str(self._counter)
        self._counter += 1
        queue: asyncio.Queue = asyncio.Queue()
        # (2) register before doing anything else, then (4) recheck liveness.
        self._pending[ident] = queue
        if not self._alive:
            self._pending.pop(ident, None)
            raise RuntimeError("plugin executor: stdin closed")

        # Serialize the request to JSON and pass it as the cli's `--request`
        # argv; the host re-enters its `run` with this command.
        argv = ["--request", json.dumps(request)]
        line = json.dumps({"type": "command", "id": ident, "command": argv})
        try:
            await self._write(line)  # (3) serialized write
        except Exception:
            self._pending.pop(ident, None)
            raise

        try:
            while True:
                item = await queue.get()
                if item is _END:
                    return
                yield item
        finally:
            self._pending.pop(ident, None)

    async def _write(self, line: str) -> None:
        async with self._write_lock:
            sys.stdout.write(line + "\n")
            sys.stdout.flush()

    def _ensure_reader(self) -> None:
        if self._reader_started:
            return
        self._reader_started = True
        self._loop = asyncio.get_running_loop()
        threading.Thread(
            target=self._read_stdin, name="objectiveai-plugin-stdin", daemon=True
        ).start()

    def _read_stdin(self) -> None:
        # Blocking readline loop on a dedicated thread; hand each line to the
        # event loop. Ends with an EOF signal when stdin closes.
        try:
            for line in sys.stdin:
                loop = self._loop
                if loop is None:
                    break
                loop.call_soon_threadsafe(self._on_line, line)
        finally:
            loop = self._loop
            if loop is not None:
                loop.call_soon_threadsafe(self._on_eof)

    def _on_line(self, line: str) -> None:
        """Route one inbound NDJSON line to the matching id's queue. Runs on the
        event loop thread (via ``call_soon_threadsafe``)."""
        line = line.strip()
        if not line:
            return
        try:
            obj = json.loads(line)
        except Exception:
            return
        if not isinstance(obj, dict):
            return
        ident = obj.get("id")
        if not isinstance(ident, str):
            return
        queue = self._pending.get(ident)
        if queue is None:
            return
        value = obj.get("value")
        # Terminal markers: SDK form {id, done:true} or host form
        # {id, value:{type:"command_complete", exit_code}}.
        if obj.get("done") is True or _is_command_complete(value):
            self._pending.pop(ident, None)
            queue.put_nowait(_END)
            return
        # Yield each value item RAW; error envelopes pass through as values and
        # the caller's CliStream discriminates them via the CliError union.
        if "value" in obj:
            queue.put_nowait(value)

    def _on_eof(self) -> None:
        # (4) flag before draining, mirroring the Rust ordering.
        self._alive = False
        for queue in list(self._pending.values()):
            queue.put_nowait(_END)
        self._pending.clear()


def _is_command_complete(value: Any) -> bool:
    return isinstance(value, dict) and value.get("type") == "command_complete"
