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

__version__ = "2.2.15"

import asyncio
import json
import os
import shutil
import sys
from pathlib import Path
from typing import Any, BinaryIO, Optional

from openai_codex_sdk import (
    Codex,
    LocalImageInput,
    TextInput,
)
from openai_codex_sdk.types import CodexOptions


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
# MCP config materialization
# ---------------------------------------------------------------------------
# Codex's Python SDK (openai-codex-sdk 0.1.x) has no typed MCP knob on
# `ThreadOptions`; both `CodexOptions` and `ThreadOptions` are pydantic
# models with `extra="forbid"`. The underlying `codex` binary, however,
# reads MCP config from `$CODEX_HOME/config.toml`, and `CodexOptions`
# does expose a public `env` knob. So we point CODEX_HOME at a
# per-request directory whose `config.toml` we write ourselves.
#
# Concurrency is safe because each request gets its own `<cwd>/.codex/`
# (the Rust client owns `cwd` and recursively deletes it when the
# stream ends), and `CodexExec` re-spawns a fresh `codex exec`
# subprocess per `Thread.run_streamed()` call, picking up the per-
# request CODEX_HOME every time.

_AUTH_LINK_LOCK = asyncio.Lock()


async def _link_auth_into(codex_home: Path) -> None:
    """Copy the user's `auth.json` into a per-request CODEX_HOME.

    Codex looks for auth in `$CODEX_HOME/auth.json`. When we point
    CODEX_HOME at a temp dir, the user's real auth file isn't visible
    unless we copy it in. Copy (not symlink) for Windows compatibility.

    No-op when the user has no auth.json on disk (e.g. they're on
    `CODEX_API_KEY`); codex falls back to env-based auth from the
    inherited `os.environ`.
    """
    real_codex_home = Path(
        os.environ.get("CODEX_HOME") or (Path.home() / ".codex")
    )
    src = real_codex_home / "auth.json"
    if not src.is_file():
        return
    dst = codex_home / "auth.json"
    if dst.exists():
        return
    async with _AUTH_LINK_LOCK:
        if dst.exists():
            return
        await asyncio.get_running_loop().run_in_executor(
            None, lambda: shutil.copy2(src, dst)
        )


def _toml_escape_basic_string(s: str) -> str:
    """Render `s` as a TOML basic string (double-quoted with escapes)."""
    out: list[str] = []
    for ch in s:
        c = ord(ch)
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\b":
            out.append("\\b")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\f":
            out.append("\\f")
        elif ch == "\r":
            out.append("\\r")
        elif c < 0x20 or c == 0x7F:
            out.append(f"\\u{c:04X}")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def _toml_quoted_key(s: str) -> str:
    """A TOML quoted key (for non-bare characters like '-' in 'Mcp-Session-Id')."""
    return _toml_escape_basic_string(s)


def _build_config_toml(mcp_servers: dict[str, Any]) -> str:
    """Serialize an mcp_servers map to a `[mcp_servers.<name>]` TOML
    fragment.

    Wire shape (from objectiveai-api Rust side):
      { "<name>": { "url": "...", "http_headers": { "<h>": "<v>" } } }

    Codex TOML schema:
      [mcp_servers.<name>]
      url = "..."
      required = true
      [mcp_servers.<name>.http_headers]
      "<h>" = "<v>"
      ...

    `required = true` is set on every entry — the user's agent
    definition explicitly opted into this MCP server, so silent
    degradation (the run starts without the tools) would be a
    correctness bug. Init failures should be loud.

    Raises:
        ValueError: when `mcp_servers` is malformed.
    """
    if not isinstance(mcp_servers, dict):
        raise ValueError("mcp_servers must be an object")

    lines: list[str] = []
    for name, cfg in mcp_servers.items():
        if not isinstance(name, str) or not name:
            raise ValueError("mcp_servers keys must be non-empty strings")
        if not isinstance(cfg, dict):
            raise ValueError(f"mcp_servers[{name!r}] must be an object")
        url = cfg.get("url")
        if not isinstance(url, str) or not url:
            raise ValueError(
                f"mcp_servers[{name!r}].url must be a non-empty string"
            )
        name_key = _toml_quoted_key(name)
        lines.append(f"[mcp_servers.{name_key}]")
        lines.append(f"url = {_toml_escape_basic_string(url)}")
        lines.append("required = true")
        http_headers = cfg.get("http_headers")
        if http_headers:
            if not isinstance(http_headers, dict):
                raise ValueError(
                    f"mcp_servers[{name!r}].http_headers must be an object"
                )
            lines.append(f"[mcp_servers.{name_key}.http_headers]")
            for hname, hval in http_headers.items():
                if not isinstance(hname, str) or not hname:
                    raise ValueError(
                        f"mcp_servers[{name!r}].http_headers keys must be non-empty strings"
                    )
                if not isinstance(hval, str):
                    raise ValueError(
                        f"mcp_servers[{name!r}].http_headers[{hname!r}] must be a string"
                    )
                lines.append(
                    f"{_toml_quoted_key(hname)} = {_toml_escape_basic_string(hval)}"
                )
        lines.append("")
    return "\n".join(lines)


async def _prepare_codex_home(
    cwd: str,
    mcp_servers: Optional[dict[str, Any]],
) -> tuple[Optional[str], Optional[dict[str, str]]]:
    """If `mcp_servers` is non-empty, materialize a per-request
    `<cwd>/.codex/` with a `config.toml` containing the MCP entries.

    Returns ``(codex_home_path, env_override)``. When `mcp_servers` is
    empty / absent, returns ``(None, None)`` — caller passes
    ``env=None`` to `CodexOptions`, which falls through to
    `os.environ` and the user's real CODEX_HOME (full
    backwards-compatibility with no-MCP runs).
    """
    if not mcp_servers:
        return None, None
    codex_home = Path(cwd) / ".codex"
    codex_home.mkdir(parents=True, exist_ok=True)
    config_toml = _build_config_toml(mcp_servers)
    config_path = codex_home / "config.toml"
    # Atomic write — codex parses this on every subprocess spawn;
    # partial writes would surface as obscure parse errors.
    tmp_path = config_path.with_suffix(".toml.tmp")
    loop = asyncio.get_running_loop()
    await loop.run_in_executor(
        None, lambda: tmp_path.write_text(config_toml, encoding="utf-8")
    )
    await loop.run_in_executor(None, lambda: tmp_path.replace(config_path))
    await _link_auth_into(codex_home)
    env_override = {**os.environ, "CODEX_HOME": str(codex_home)}
    return str(codex_home), env_override


# ---------------------------------------------------------------------------
# Per-request handler
# ---------------------------------------------------------------------------


def _one_line(msg: str) -> str:
    """Flatten an error message to a single line for safe NDJSON embedding."""
    return " ".join(msg.split())


def _only_set(d: dict[str, Any]) -> dict[str, Any]:
    return {k: v for k, v in d.items() if v is not None}


async def handle_run(
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

        # MCP: when params.mcp_servers is non-empty, materialize a
        # per-request `<cwd>/.codex/config.toml` and point the codex
        # subprocess at it via CODEX_HOME. When empty/absent, this is
        # a no-op and codex inherits the user's real CODEX_HOME.
        _codex_home, env_override = await _prepare_codex_home(
            params["cwd"], params.get("mcp_servers")
        )
        # Propagate the composite agent id into the codex subprocess
        # env so any objectiveai cli invocation it makes via the
        # filesystem sees OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY. Builds env_override
        # on demand when MCP didn't already need it — both go through
        # the same CodexOptions(env=...) channel.
        agent_instance_hierarchy = params.get("agent_instance_hierarchy")
        if agent_instance_hierarchy:
            if env_override is None:
                env_override = {**os.environ}
            env_override["OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY"] = agent_instance_hierarchy
        codex_options = (
            CodexOptions(env=env_override) if env_override is not None else None
        )
        codex = Codex(codex_options)

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
    mcp_servers = params.get("mcp_servers")
    if mcp_servers is not None and not isinstance(mcp_servers, dict):
        return "'mcp_servers' must be an object"
    return None


async def _dispatch(
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

    # `Codex()` is constructed per request inside `handle_run` — its
    # __init__ does no I/O (only resolves the binary path and stores
    # options), so per-request construction is essentially free, and
    # it lets each request inject its own CODEX_HOME for MCP config
    # without sharing state across concurrent runs.

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
        await _dispatch(msg, tasks, stdout_writer, stderr_writer)

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
