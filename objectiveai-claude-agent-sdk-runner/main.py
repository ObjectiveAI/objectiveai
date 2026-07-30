#!/usr/bin/env python3
"""ObjectiveAI Claude Agent SDK Runner — stdio NDJSON server.

A long-lived process that accepts multiple concurrent Claude Agent SDK
runs over a single stdin/stdout/stderr pair. The caller multiplexes by
attaching a string ``id`` to every request; every line emitted on
stdout and stderr carries that same ``id`` so the caller can
demultiplex events from N concurrent streams.

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
    {"type":"cancel","id":"<id>"}                    # abort one stream

  Outbound (stdout):
    {"type":"event","id":"<id>","event":{...}}       # one SDK message
    {"type":"end","id":"<id>","status":"ok"}
    {"type":"end","id":"<id>","status":"cancelled"}
    {"type":"end","id":"<id>","status":"error","error":"<msg>"}

  Outbound (stderr) during operation:
    {"type":"diag","id":"<id>","level":"warn","message":"..."}

  Outbound (stderr) before main_loop is up — process-fatal only:
    {"type":"fatal","message":"..."}                 # untagged carve-out

EOF on stdin = drain every in-flight task, exit 0. There is no
``shutdown`` message — the runner cannot be told to kill the whole
process via JSON; the only stop paths are EOF or external signal.

Concurrency: every emit on stdout (and on stderr) is serialized through
one ``asyncio.Lock`` per stream. This keeps line bytes from
interleaving across coroutines, but it also means a slow consumer will
block ALL in-flight runs once the OS pipe buffer fills. The caller
MUST drain stdout promptly.
"""

from __future__ import annotations

__version__ = "2.2.14"

import asyncio
import json
import os
import sys
import time
from dataclasses import replace
from typing import Any, BinaryIO, Optional

from claude_agent_sdk import ClaudeAgentOptions, ClaudeSDKClient
from claude_agent_sdk.types import (
    AssistantMessage,
    Message,
    RateLimitEvent,
    ResultMessage,
    StreamEvent,
    SystemMessage,
    TaskNotificationMessage,
    TaskProgressMessage,
    TaskStartedMessage,
    TextBlock,
    ThinkingBlock,
    ToolResultBlock,
    ToolUseBlock,
    UserMessage,
)

def _resolve_claude_exe() -> Optional[str]:
    """Resolve the NATIVE claude.exe, bypassing the npm `.cmd`/sh shim that the
    SDK's ``shutil.which("claude")`` finds. On Windows the shim spawns but then
    fails to locate its target binary in some launched environments ("The
    system cannot find the file specified"); pointing ``cli_path`` straight at
    the real exe avoids that entirely. Returns None if not found (the SDK then
    falls back to its own discovery)."""
    import shutil

    direct = shutil.which("claude.exe")
    if direct and os.path.isfile(direct):
        return direct
    cands = []
    shim = shutil.which("claude")
    if shim:
        cands.append(
            os.path.join(
                os.path.dirname(shim),
                "node_modules", "@anthropic-ai", "claude-code", "bin", "claude.exe",
            )
        )
    cands.append(
        os.path.join(
            os.path.expanduser("~"),
            "AppData", "Roaming", "npm",
            "node_modules", "@anthropic-ai", "claude-code", "bin", "claude.exe",
        )
    )
    for c in cands:
        if os.path.isfile(c):
            return c
    return None


_CLAUDE_EXE = _resolve_claude_exe()


# ---------------------------------------------------------------------------
# Session cwd
# ---------------------------------------------------------------------------
# The claude CLI scopes saved conversations to a PROJECT derived from the
# process cwd (`~/.claude/projects/<sanitized-cwd>/<session-id>.jsonl`).
# `--resume <id>` only finds a session in the project matching the CURRENT
# cwd. The runner is spawned by the API with whatever cwd the caller
# happened to have, so a create at cwd A and a resume at cwd B looked in
# different projects → "No conversation found with session ID". Pin every
# run to a single stable cwd (home — where existing sessions were created)
# so create and resume always share one project.
_SESSION_CWD = os.path.expanduser("~")


# ---------------------------------------------------------------------------
# Wire-format serialization
# ---------------------------------------------------------------------------
# The Python SDK parses raw JSONL dicts from the CLI into typed dataclass
# objects. We need to serialize them back to the wire format so the Rust
# caller (objectiveai-api) can deserialize them as SDKMessage.


def _serialize_content_block(block: Any) -> dict[str, Any]:
    if isinstance(block, TextBlock):
        return {"type": "text", "text": block.text}
    if isinstance(block, ThinkingBlock):
        return {
            "type": "thinking",
            "thinking": block.thinking,
            "signature": block.signature,
        }
    if isinstance(block, ToolUseBlock):
        return {
            "type": "tool_use",
            "id": block.id,
            "name": block.name,
            "input": block.input,
        }
    if isinstance(block, ToolResultBlock):
        d: dict[str, Any] = {
            "type": "tool_result",
            "tool_use_id": block.tool_use_id,
        }
        if block.content is not None:
            d["content"] = block.content
        if block.is_error is not None:
            d["is_error"] = block.is_error
        return d
    # Unknown block type — pass through as-is if it's already a dict
    if isinstance(block, dict):
        return block
    return {}


def serialize_message(msg: Message) -> dict[str, Any] | None:
    """Convert a typed Message back to the wire-format dict for JSONL output.

    SystemMessage (and its task subtypes) stores the original raw dict in its
    ``data`` attribute, so we can return it directly without reconstruction.
    """
    # Task subtypes must be checked before SystemMessage (they inherit from it).
    if isinstance(
        msg, (TaskStartedMessage, TaskProgressMessage, TaskNotificationMessage)
    ):
        return msg.data
    if isinstance(msg, SystemMessage):
        return msg.data

    if isinstance(msg, AssistantMessage):
        content = [_serialize_content_block(b) for b in msg.content]
        message_inner: dict[str, Any] = {
            "role": "assistant",
            "content": content,
            "model": msg.model,
        }
        if msg.usage is not None:
            message_inner["usage"] = msg.usage
        if msg.message_id is not None:
            message_inner["id"] = msg.message_id
        if msg.stop_reason is not None:
            message_inner["stop_reason"] = msg.stop_reason

        d: dict[str, Any] = {"type": "assistant", "message": message_inner}
        if msg.session_id is not None:
            d["session_id"] = msg.session_id
        if msg.uuid is not None:
            d["uuid"] = msg.uuid
        if msg.parent_tool_use_id is not None:
            d["parent_tool_use_id"] = msg.parent_tool_use_id
        if msg.error is not None:
            d["error"] = msg.error
        return d

    if isinstance(msg, UserMessage):
        if isinstance(msg.content, str):
            content_val: Any = msg.content
        else:
            content_val = [_serialize_content_block(b) for b in msg.content]
        d = {"type": "user", "message": {"role": "user", "content": content_val}}
        if msg.uuid is not None:
            d["uuid"] = msg.uuid
        if msg.parent_tool_use_id is not None:
            d["parent_tool_use_id"] = msg.parent_tool_use_id
        if msg.tool_use_result is not None:
            d["tool_use_result"] = msg.tool_use_result
        return d

    if isinstance(msg, ResultMessage):
        d = {
            "type": "result",
            "subtype": msg.subtype,
            "duration_ms": msg.duration_ms,
            "duration_api_ms": msg.duration_api_ms,
            "is_error": msg.is_error,
            "num_turns": msg.num_turns,
            "session_id": msg.session_id,
        }
        if msg.stop_reason is not None:
            d["stop_reason"] = msg.stop_reason
        if msg.total_cost_usd is not None:
            d["total_cost_usd"] = msg.total_cost_usd
        if msg.usage is not None:
            d["usage"] = msg.usage
        if msg.result is not None:
            d["result"] = msg.result
        if msg.structured_output is not None:
            d["structured_output"] = msg.structured_output
        if msg.model_usage is not None:
            d["modelUsage"] = msg.model_usage
        if msg.uuid is not None:
            d["uuid"] = msg.uuid
        return d

    if isinstance(msg, StreamEvent):
        d = {
            "type": "stream_event",
            "uuid": msg.uuid,
            "session_id": msg.session_id,
            "event": msg.event,
        }
        if msg.parent_tool_use_id is not None:
            d["parent_tool_use_id"] = msg.parent_tool_use_id
        return d

    if isinstance(msg, RateLimitEvent):
        info = msg.rate_limit_info
        rate_info: dict[str, Any] = {"status": info.status}
        if info.resets_at is not None:
            rate_info["resetsAt"] = info.resets_at
        if info.rate_limit_type is not None:
            rate_info["rateLimitType"] = info.rate_limit_type
        if info.utilization is not None:
            rate_info["utilization"] = info.utilization
        if info.overage_status is not None:
            rate_info["overageStatus"] = info.overage_status
        if info.overage_resets_at is not None:
            rate_info["overageResetsAt"] = info.overage_resets_at
        if info.overage_disabled_reason is not None:
            rate_info["overageDisabledReason"] = info.overage_disabled_reason
        return {
            "type": "rate_limit_event",
            "uuid": msg.uuid,
            "session_id": msg.session_id,
            "rate_limit_info": rate_info,
        }

    # Unknown message type — skip.
    return None


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
# MCP readiness
# ---------------------------------------------------------------------------


async def wait_for_mcp_servers(
    client: ClaudeSDKClient,
    our_servers: set[str],
) -> None:
    """Poll MCP server status until all configured servers are connected."""
    if not our_servers:
        return

    first = True
    delay = 0.001  # 1ms initial, doubles up to 100ms
    while True:
        status = await client.get_mcp_status()
        servers = status.get("mcpServers", [])

        if first:
            status_names = {s["name"] for s in servers}
            missing = our_servers - status_names
            if missing:
                raise RuntimeError(
                    f"MCP servers not found in status list: {', '.join(sorted(missing))}. "
                    f"Available: {', '.join(sorted(status_names))}"
                )
            first = False

        pending = False
        for s in servers:
            if s["name"] not in our_servers:
                continue
            st = s.get("status", "")
            if st in ("failed", "needs-auth"):
                error = s.get("error", "")
                raise RuntimeError(
                    f"MCP server {s['name']}: {st}" + (f" - {error}" if error else "")
                )
            if st == "pending":
                pending = True

        if not pending:
            break

        await asyncio.sleep(delay)
        if delay < 0.1:
            delay *= 2


# ---------------------------------------------------------------------------
# Per-request handler
# ---------------------------------------------------------------------------


def _one_line(msg: str) -> str:
    """Flatten an error message to a single line for safe NDJSON embedding."""
    return " ".join(msg.split())


async def handle_run(
    request_id: str,
    params: dict[str, Any],
    stdout_writer: Writer,
    stderr_writer: Writer,
) -> None:
    """Run one Claude Agent SDK conversation, tagging every emit with
    ``request_id``. Always emits exactly one terminal ``end`` line via
    ``asyncio.shield`` even on cancellation.

    The Rust caller enforces the FIFO concurrency cap on its side, so
    every ``run`` that reaches this function already holds a slot —
    there is nothing to wait for here.
    """
    status: str = "ok"
    error: Optional[str] = None
    try:
        message: dict[str, Any] = params["message"]
        mcp_servers: dict[str, Any] = params.get("mcp_servers") or {}

        # Build thinking config.
        thinking = None
        if params.get("thinking_disabled"):
            thinking = {"type": "disabled"}

        # Build env overrides — routed to ClaudeAgentOptions.env, NOT
        # os.environ. Concurrent runs with different user_agents are
        # therefore isolated per-subprocess.
        env: dict[str, str] = {}
        ua = params.get("user_agent")
        if ua:
            env["CLAUDE_AGENT_SDK_CLIENT_APP"] = ua
        agent_instance_hierarchy = params.get("agent_instance_hierarchy")
        if agent_instance_hierarchy:
            # Propagates through any objectiveai cli invocation the
            # subprocess makes via the filesystem — the cli reads
            # OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY at startup and stamps it on every
            # outgoing X-OBJECTIVEAI-AGENT-INSTANCE-HIERARCHY header.
            env["OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY"] = agent_instance_hierarchy

        opts = ClaudeAgentOptions(
            model=params["model"],
            # Pin cwd so claude's per-project session store is stable across
            # create + resume (else `--resume` can't find the session).
            cwd=_SESSION_CWD,
            system_prompt=params.get("system_prompt"),
            effort=params.get("effort"),
            thinking=thinking,
            mcp_servers=mcp_servers,
            resume=params.get("resume"),
            env=env,
            tools=[],
            include_partial_messages=True,
            permission_mode="bypassPermissions",
            cli_path=_CLAUDE_EXE,
            # Isolate the headless agent from the operator's global ~/.claude
            # config + account connectors. Without these the agent inherits the
            # user's interactive connectors (Gmail/Calendar/Drive) ALONGSIDE the
            # SDK-provided `oaip` MCP, and the SDK's MCP tools never register —
            # the agent reports "no X tool". `strict_mcp_config` makes claude
            # use ONLY the mcp_servers passed here (verified: drops the account
            # connectors); `setting_sources=[]` drops settings-file tools.
            setting_sources=[],
            strict_mcp_config=True,
        )

        # Async generator yielding the single SDK user message.
        async def messages():
            yield message

        our_servers = set(mcp_servers.keys())

        max_retries = int(params["rate_limit_max_retries"])
        max_wait_secs = int(params.get("rate_limit_max_wait_secs", 180))
        current_session_id: str | None = None

        for attempt in range(max_retries + 1):
            rate_limited = False

            # On retry, resume the session captured from the previous attempt.
            if current_session_id is not None:
                opts = replace(opts, resume=current_session_id)

            async with ClaudeSDKClient(opts) as client:
                # Only send the original message on the first attempt.
                # On resume, the session already has the conversation
                # history.
                if attempt == 0:
                    await client.connect(prompt=messages())
                else:
                    await client.connect()

                await wait_for_mcp_servers(client, our_servers)

                async for msg in client.receive_messages():
                    # Track session_id from any message that has it.
                    msg_session_id = getattr(msg, "session_id", None)
                    if msg_session_id:
                        current_session_id = msg_session_id

                    if (
                        isinstance(msg, RateLimitEvent)
                        and msg.rate_limit_info.status == "rejected"
                    ):
                        resets_at = msg.rate_limit_info.resets_at
                        if resets_at is not None and attempt < max_retries:
                            wait = max(0, resets_at - time.time()) + 1
                            if wait > max_wait_secs:
                                await stderr_writer.emit_diag(
                                    request_id,
                                    "warn",
                                    f"Rate limited, but wait {wait:.0f}s "
                                    f"exceeds max {max_wait_secs}s — giving up",
                                )
                                # Fall through to emit the event and
                                # exit the outer retry loop on this
                                # attempt.
                            else:
                                await stderr_writer.emit_diag(
                                    request_id,
                                    "warn",
                                    f"Rate limited, retrying in {wait:.0f}s "
                                    f"(attempt {attempt + 1}/{max_retries})",
                                )
                                await asyncio.sleep(wait)
                                rate_limited = True
                                break
                    d = serialize_message(msg)
                    if d is not None:
                        await stdout_writer.emit_event(request_id, d)

            if not rate_limited:
                break
    except asyncio.CancelledError:
        # Reachable only on subprocess SIGTERM during the drain phase
        # (in-flight cancellation isn't part of the wire protocol — the
        # Claude Agent SDK can't guarantee a stop point that doesn't
        # leave a billing event unaccounted for, so we let queries run
        # to natural completion). Surface the abort as an error so the
        # consumer sees that the work didn't finish.
        status = "error"
        error = "cancelled"
        # Re-raised in the finally after we emit the terminal end line.
        raise
    except Exception as e:
        status = "error"
        error = _one_line(str(e) or e.__class__.__name__)
    finally:
        # Shield the terminal emit so that a second cancel arriving while
        # the SDK's __aexit__ is unwinding doesn't suppress it.
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


_REQUIRED_PARAMS = ("model", "message", "rate_limit_max_retries")


def _validate_run_params(params: Any) -> Optional[str]:
    """Return None if params is acceptable, else a one-line error string."""
    if not isinstance(params, dict):
        return "params must be an object"
    for key in _REQUIRED_PARAMS:
        if key not in params:
            return f"missing required field '{key}'"
    if not isinstance(params["model"], str) or not params["model"]:
        return "'model' must be a non-empty string"
    if not isinstance(params["message"], dict):
        return "'message' must be an object"
    try:
        int(params["rate_limit_max_retries"])
    except (TypeError, ValueError):
        return "'rate_limit_max_retries' must be an integer"
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
            # Defensive: if the task somehow finished with an uncaught
            # non-CancelledError, report it. Should be unreachable given
            # handle_run's try/finally.
            if not t.cancelled():
                exc = t.exception()
                if exc is not None and not isinstance(exc, asyncio.CancelledError):
                    # Best-effort, fire-and-forget.
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

    # Note: there is no `cancel` handler. In-flight cancellation isn't
    # supported because the Claude Agent SDK can't guarantee a stop
    # point that doesn't leave a billing event unaccounted for. A
    # `cancel`-typed line falls into the unknown-type handler below.

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
        # Avoid conflicts with the SDK-spawned `claude` CLI.
        os.environ.pop("CLAUDECODE", None)
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
