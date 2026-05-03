#!/usr/bin/env python3
"""ObjectiveAI Codex SDK Runner — stdio NDJSON server.

A long-lived process that accepts multiple concurrent Codex SDK runs
over a single stdin/stdout/stderr pair. The caller multiplexes by
attaching a string ``id`` to every request; every line emitted on
stdout and stderr carries that same ``id`` so the caller can
demultiplex events from N concurrent streams.

Authentication is inherited from the user's ``~/.codex/auth.json``
(written by ``codex login``). The SDK shells out to the ``codex``
binary which reads that file — we do nothing special for auth.

Spawn with no arguments:

  $ runner

The runner has no built-in concurrency cap. The Rust caller
(objectiveai-api) enforces a FIFO ``query_limit`` on its side via a
``tokio::sync::Semaphore``, so surplus requests never reach this
process — they wait for a slot before the ``run`` line is sent.

Wire protocol — NDJSON, one JSON object per line, UTF-8, terminated by
``\\n``:

  Inbound (stdin):
    {"type":"run","id":"<id>","params":{...}}        # start a stream

  Outbound (stdout):
    {"type":"event","id":"<id>","event":{...}}       # one ThreadEvent
    {"type":"end","id":"<id>","status":"ok"}
    {"type":"end","id":"<id>","status":"error","error":"<msg>"}

  Outbound (stderr) during operation:
    {"type":"diag","id":"<id>","level":"warn","message":"..."}

  Outbound (stderr) before main_loop is up — process-fatal only:
    {"type":"fatal","message":"..."}                 # untagged carve-out

EOF on stdin = drain every in-flight task, exit 0. There is no
``cancel`` message — codex's ``Thread`` doesn't expose a stop point
that doesn't leave a billing event unaccounted for, so we run every
accepted ``run`` to natural completion.

Concurrency: every emit on stdout (and on stderr) is serialized
through one ``asyncio.Lock``. This keeps line bytes from interleaving
across coroutines. The caller MUST drain stdout promptly or the OS
pipe buffer fills and ALL in-flight runs block.
"""

from __future__ import annotations

__version__ = "2.0.0"

import asyncio
import json
import sys
from typing import Any, BinaryIO, Optional

from openai_codex_sdk import (
    Codex,
    LocalImageInput,
    TextInput,
)


# ---------------------------------------------------------------------------
# Input parsing
# ---------------------------------------------------------------------------
# `params.input` is a JSON object representing a single user message:
#
#   {
#     "content": "string"                     # plain text content, OR
#                | [                          # an ordered list of parts:
#                  {"type": "text", "text": "..."},
#                  {"type": "local_image", "path": "..."}
#                ],
#     "name": "optional-author-name"          # rendered as a "[name] :"
#                                             # prefix text part if present
#   }


def _parse_input_payload(raw: Any) -> list[Any]:
    """Translate the run-line ``params.input`` value into Codex SDK input items."""
    if not isinstance(raw, dict):
        raise ValueError("params.input must be an object representing a user message")

    name = raw.get("name")
    if name is not None and not isinstance(name, str):
        raise ValueError("params.input.name must be a string")

    content = raw.get("content")
    if content is None:
        raise ValueError("params.input is missing required field: content")

    items: list[Any] = []

    # Optional name → leading "[name] :" text part, mirroring Claude's prompt.rs.
    if isinstance(name, str) and name:
        items.append(TextInput(type="text", text=f"[{name}] :"))

    if isinstance(content, str):
        items.append(TextInput(type="text", text=content))
    elif isinstance(content, list):
        for idx, part in enumerate(content):
            if not isinstance(part, dict) or "type" not in part:
                raise ValueError(
                    f"params.input.content[{idx}] must be an object with a 'type' field"
                )
            t = part["type"]
            try:
                if t == "text":
                    items.append(TextInput(type="text", text=part["text"]))
                elif t == "local_image":
                    items.append(
                        LocalImageInput(type="local_image", path=part["path"])
                    )
                else:
                    raise ValueError(
                        f"params.input.content[{idx}] has unknown type: {t!r}"
                    )
            except KeyError as e:
                raise ValueError(
                    f"params.input.content[{idx}] missing required field: {e.args[0]}"
                )
    else:
        raise ValueError("params.input.content must be a string or an array of parts")

    return items


# ---------------------------------------------------------------------------
# Stream writer (atomic, lock-serialized line emission)
# ---------------------------------------------------------------------------


class Writer:
    """Serializes async writes to one binary stream.

    Every emit is a single UTF-8 line ending in ``\\n``, written-and-flushed
    under one ``asyncio.Lock`` so concurrent coroutines never interleave
    bytes mid-line. We go through ``raw.write`` on the binary buffer
    (``sys.stdout.buffer`` / ``sys.stderr.buffer``) to bypass Python's
    text-mode ``\\n``→``\\r\\n`` translation on Windows.
    """

    def __init__(self, raw: BinaryIO) -> None:
        self._raw = raw
        self._lock = asyncio.Lock()

    async def emit(self, payload: dict) -> None:
        line = json.dumps(payload, separators=(",", ":"), ensure_ascii=False) + "\n"
        data = line.encode("utf-8")
        async with self._lock:
            self._raw.write(data)
            self._raw.flush()

    # --- stdout helpers ---

    async def emit_event(self, request_id: str, event: dict) -> None:
        await self.emit({"type": "event", "id": request_id, "event": event})

    async def emit_end(
        self,
        request_id: str,
        status: str,
        error: Optional[str] = None,
    ) -> None:
        payload: dict[str, Any] = {
            "type": "end",
            "id": request_id,
            "status": status,
        }
        if error is not None:
            payload["error"] = error
        await self.emit(payload)

    # --- stderr helper ---

    async def emit_diag(self, request_id: str, level: str, message: str) -> None:
        await self.emit({
            "type": "diag",
            "id": request_id,
            "level": level,
            "message": message,
        })


# ---------------------------------------------------------------------------
# Event serialization
# ---------------------------------------------------------------------------


def _serialize_event(event: Any) -> dict[str, Any]:
    """Convert a typed Codex SDK ThreadEvent to the wire-format dict."""
    if hasattr(event, "model_dump"):
        return event.model_dump(mode="json", by_alias=False, exclude_none=False)
    if isinstance(event, dict):
        return event
    return {"_repr": repr(event)}


# ---------------------------------------------------------------------------
# Per-request handler
# ---------------------------------------------------------------------------


def _one_line(msg: str) -> str:
    """Flatten an error message to a single line for safe NDJSON embedding."""
    return " ".join(msg.split())


def _only_set(d: dict[str, Any]) -> dict[str, Any]:
    return {k: v for k, v in d.items() if v is not None}


async def handle_run(
    codex: Codex,
    request_id: str,
    params: dict[str, Any],
    stdout_writer: Writer,
    stderr_writer: Writer,
) -> None:
    """Run one Codex SDK turn, tagging every emit with ``request_id``.

    Always emits exactly one terminal ``end`` line via ``asyncio.shield``
    even on cancellation.
    """
    status: str = "ok"
    error: Optional[str] = None
    try:
        # Hardcoded sandboxing posture for headless / untrusted operation:
        # - working_directory: provided by the caller via params.cwd. The
        #   Rust client owns the tempdir lifecycle so any image inputs it
        #   materialized stay alive for the run.
        # - sandbox_mode = "read-only": no filesystem mutations.
        # - approval_policy = "untrusted": codex never auto-approves anything.
        # - skip_git_repo_check = True: the cwd isn't a git repo and
        #   shouldn't need to be.
        thread_options = _only_set({
            "model": params.get("model"),
            "model_reasoning_effort": params.get("effort"),
            "web_search_enabled": params.get("web_search_enabled"),
            "working_directory": params.get("cwd"),
            "sandbox_mode": "read-only",
            "approval_policy": "untrusted",
            "skip_git_repo_check": True,
        })

        # NOTE(MCP): params.get("mcp_servers") is intentionally ignored
        # for now — the codex Python SDK Thread API has no MCP knob.
        # The Rust side plumbs this field through end-to-end so the wire
        # protocol is stable; wiring it into Codex.Thread is a follow-up.

        if params.get("resume"):
            thread = codex.resume_thread(params["resume"], thread_options)
        else:
            thread = codex.start_thread(thread_options)

        input_ = _parse_input_payload(params.get("input"))
        streamed = await thread.run_streamed(input_)

        async for event in streamed.events:
            await stdout_writer.emit_event(request_id, _serialize_event(event))
            event_type = getattr(event, "type", None)
            if event_type == "turn.failed" or event_type == "error":
                status = "error"
                error = _one_line(
                    f"thread run {event_type}: "
                    + getattr(getattr(event, "error", None), "message", "")
                    or getattr(event, "message", "")
                    or event_type
                )
                # Don't break — let the SDK finish its event stream so
                # any trailing usage event is preserved. The terminal
                # `end` line will carry the error.
    except asyncio.CancelledError:
        # Reachable only on subprocess SIGTERM during the drain phase.
        # In-flight cancellation isn't part of the wire protocol.
        status = "error"
        error = "cancelled"
        raise
    except Exception as e:
        status = "error"
        error = _one_line(str(e) or e.__class__.__name__)
    finally:
        # Shield the terminal emit so that a second cancel arriving
        # while the SDK is unwinding doesn't suppress it.
        try:
            await asyncio.shield(stdout_writer.emit_end(request_id, status, error))
        except Exception:
            pass


# ---------------------------------------------------------------------------
# Stdin reader
# ---------------------------------------------------------------------------


async def read_one_line_from_stdin() -> Optional[str]:
    """Read one line from stdin without blocking the event loop.

    Uses a thread-pool executor instead of ``loop.connect_read_pipe`` because
    ProactorEventLoop on Windows does not handle stdin-as-pipe reliably. The
    blocking ``readline`` runs on a worker thread; on EOF the thread returns
    naturally and the next iteration sees ``None``.
    """
    loop = asyncio.get_running_loop()
    raw = await loop.run_in_executor(None, sys.stdin.buffer.readline)
    if not raw:
        return None
    return raw.decode("utf-8", errors="replace")


# ---------------------------------------------------------------------------
# Dispatcher
# ---------------------------------------------------------------------------


_REQUIRED_PARAMS = ("model", "input", "cwd")


def _validate_run_params(params: Any) -> Optional[str]:
    """Return None if params is acceptable, else a one-line error string."""
    if not isinstance(params, dict):
        return "params must be an object"
    for key in _REQUIRED_PARAMS:
        if key not in params:
            return f"missing required field '{key}'"
    if not isinstance(params["model"], str) or not params["model"]:
        return "'model' must be a non-empty string"
    if not isinstance(params["input"], dict):
        return "'input' must be an object"
    if not isinstance(params["cwd"], str) or not params["cwd"]:
        return "'cwd' must be a non-empty string"
    return None


async def _dispatch(
    codex: Codex,
    msg: dict,
    tasks: dict[str, asyncio.Task],
    stdout_writer: Writer,
    stderr_writer: Writer,
) -> None:
    msg_type = msg.get("type")
    request_id = msg.get("id")

    if msg_type == "run":
        # No id — drop silently (no tag to attach output to).
        if not isinstance(request_id, str) or not request_id:
            return
        if request_id in tasks:
            await stdout_writer.emit_end(request_id, "error", "duplicate-id")
            return
        params = msg.get("params")
        validation_error = _validate_run_params(params)
        if validation_error is not None:
            await stdout_writer.emit_end(
                request_id, "error", f"invalid-params: {validation_error}"
            )
            return
        task = asyncio.create_task(
            handle_run(
                codex,
                request_id,
                params,
                stdout_writer,
                stderr_writer,
            )
        )
        tasks[request_id] = task

        def _done(t: asyncio.Task, _id: str = request_id) -> None:
            tasks.pop(_id, None)
            if not t.cancelled():
                exc = t.exception()
                if exc is not None and not isinstance(exc, asyncio.CancelledError):
                    try:
                        loop = asyncio.get_event_loop()
                        loop.create_task(
                            stderr_writer.emit_diag(
                                _id,
                                "error",
                                f"internal task error: {_one_line(str(exc))}",
                            )
                        )
                    except Exception:
                        pass

        task.add_done_callback(_done)
        return

    # Note: no `cancel` handler. In-flight cancellation isn't supported.

    # Unknown type — emit a tagged error if we have an id; otherwise drop.
    if isinstance(request_id, str) and request_id:
        await stdout_writer.emit_end(
            request_id, "error", f"unknown-type: {msg_type!r}"
        )


# ---------------------------------------------------------------------------
# Main loop
# ---------------------------------------------------------------------------


async def main_loop() -> None:
    stdout_writer = Writer(sys.stdout.buffer)
    stderr_writer = Writer(sys.stderr.buffer)
    tasks: dict[str, asyncio.Task] = {}

    # One Codex client shared across every in-flight request — it's a
    # thin wrapper around the codex binary; instantiation is cheap but
    # not free, and concurrent threads can be created from one Codex.
    codex = Codex()

    while True:
        line = await read_one_line_from_stdin()
        if line is None:
            break  # EOF → drain phase
        line = line.strip()
        if not line:
            continue
        try:
            msg = json.loads(line)
        except json.JSONDecodeError:
            # Process is alive and not dying — silently drop. Untagged
            # output is forbidden during normal operation.
            continue
        if not isinstance(msg, dict):
            continue
        await _dispatch(codex, msg, tasks, stdout_writer, stderr_writer)

    # Drain phase: every in-flight task emits its own end line as it
    # finishes or is cancelled.
    if tasks:
        await asyncio.gather(*list(tasks.values()), return_exceptions=True)


# ---------------------------------------------------------------------------
# Windows asyncio shutdown shim
# ---------------------------------------------------------------------------


def _silence_proactor_pipe_warnings() -> None:
    """Workaround for a long-standing CPython bug on Windows.

    On Windows, asyncio's `_ProactorBasePipeTransport` and
    `BaseSubprocessTransport` `__del__` methods run during interpreter
    shutdown and may try to `repr` themselves for a debug log;
    `__repr__` calls `fileno()` on the underlying pipe; if the pipe is
    already closed (which is *normal* at shutdown — the OS or the SDK
    subprocess we drove has gone away) `fileno()` raises
    `ValueError: I/O operation on closed pipe`. Python prints the trace
    as `Exception ignored in:` on stderr.

    Two distinct entry points hit this:
      - `_ProactorBasePipeTransport.__del__` directly.
      - `BaseSubprocessTransport.__del__` → `__repr__` → child pipe's
        `_ProactorBasePipeTransport.__repr__` → `fileno()`.

    Both must be wrapped — patching only the pipe-transport's `__del__`
    leaves the subprocess-transport path open. The exception is
    harmless (the transport already released everything it owned) but
    anything downstream that treats stderr as a failure signal (e.g.
    objectiveai-api wrapping our exit) sees it and reports a 500.

    Refs: bpo-39232, gh-91555.
    """
    if sys.platform != "win32":
        return

    def _wrap_del(cls: Any) -> None:
        original = cls.__del__

        def _patched(self, *a: Any, **kw: Any) -> None:
            try:
                original(self, *a, **kw)
            except (ValueError, OSError):
                # Closed-pipe race during shutdown — drop it. Anything
                # else propagates so real bugs still surface.
                pass

        cls.__del__ = _patched  # type: ignore[method-assign]

    try:
        from asyncio.proactor_events import _ProactorBasePipeTransport  # type: ignore[attr-defined]
        _wrap_del(_ProactorBasePipeTransport)
    except Exception:
        pass
    try:
        from asyncio.base_subprocess import BaseSubprocessTransport
        _wrap_del(BaseSubprocessTransport)
    except Exception:
        pass


# ---------------------------------------------------------------------------
# Process entrypoint
# ---------------------------------------------------------------------------


def _emit_pre_startup_fatal(message: str) -> None:
    """Untagged stderr line — used ONLY when the process is about to exit
    non-zero before the main loop can come up. The ``"type":"fatal"``
    discriminator lets the caller distinguish this from per-request
    diagnostics."""
    line = json.dumps(
        {"type": "fatal", "message": _one_line(message)}, ensure_ascii=False
    ) + "\n"
    try:
        sys.stderr.buffer.write(line.encode("utf-8"))
        sys.stderr.buffer.flush()
    except Exception:
        pass


def main() -> None:
    try:
        _silence_proactor_pipe_warnings()
    except Exception as e:
        _emit_pre_startup_fatal(f"startup: {e}")
        sys.exit(1)

    try:
        asyncio.run(main_loop())
    except Exception as e:
        _emit_pre_startup_fatal(f"main_loop: {e}")
        sys.exit(1)
    sys.exit(0)


if __name__ == "__main__":
    main()
