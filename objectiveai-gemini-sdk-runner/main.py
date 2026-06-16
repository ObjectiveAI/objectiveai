#!/usr/bin/env python3
"""ObjectiveAI Gemini SDK Runner — stdio NDJSON server.

A long-lived process that accepts multiple concurrent Google Gen AI SDK
(``google-genai``) runs over a single stdin/stdout/stderr pair. The
caller multiplexes by attaching a string ``id`` to every request; every
line emitted on stdout and stderr carries that same ``id`` so the caller
can demultiplex events from N concurrent streams.

Authentication is inherited from the environment: ``GEMINI_API_KEY`` /
``GOOGLE_API_KEY`` are read by ``genai.Client()`` directly — we do
nothing special for auth.

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
    {"type":"event","id":"<id>","event":{...}}       # one inner event
    {"type":"end","id":"<id>","status":"ok"}
    {"type":"end","id":"<id>","status":"error","error":"<msg>"}

  Outbound (stderr) during operation:
    {"type":"diag","id":"<id>","level":"warn","message":"..."}

  Outbound (stderr) before main_loop is up — process-fatal only:
    {"type":"fatal","message":"..."}                 # untagged carve-out

The inner ``event`` objects emitted on the ``{"type":"event"}`` channel
are one of (the API parses these):

  {"kind":"text","text":".."}                        # assistant text delta
  {"kind":"thinking","text":".."}                    # thought delta
  {"kind":"tool_use","id":"<call_id>","name":"..","input":{..}}
  {"kind":"tool_result","tool_use_id":"<call_id>","content":"<text>","is_error":bool}
  {"kind":"usage","input_tokens":N,"output_tokens":N,
   "thinking_tokens":N,"total_tokens":N}             # emitted once at the end

EOF on stdin = drain every in-flight task, exit 0. There is no
``cancel`` message — in-flight cancellation isn't part of the wire
protocol, so we run every accepted ``run`` to natural completion.

Concurrency: every emit on stdout (and on stderr) is serialized
through one ``asyncio.Lock``. This keeps line bytes from interleaving
across coroutines. The caller MUST drain stdout promptly or the OS
pipe buffer fills and ALL in-flight runs block.
"""

from __future__ import annotations

__version__ = "2.2.2"

import asyncio
import json
import os
import sys
from contextlib import AsyncExitStack
from typing import Any, BinaryIO, Optional

from google import genai
from google.genai import types
from mcp import ClientSession
from mcp.client.streamable_http import streamablehttp_client


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
# Misc helpers
# ---------------------------------------------------------------------------


def _one_line(msg: str) -> str:
    """Flatten an error message to a single line for safe NDJSON embedding."""
    return " ".join(msg.split())


def _coerce_int(value: Any) -> int:
    """Treat None (and anything non-int) as 0 for usage accounting."""
    if isinstance(value, bool):
        return int(value)
    if isinstance(value, int):
        return value
    try:
        return int(value)
    except (TypeError, ValueError):
        return 0


def _stringify_tool_content(content: Any) -> str:
    """Flatten an MCP ``call_tool`` result into a single text string.

    The MCP Python SDK returns a ``CallToolResult`` whose ``content`` is a
    list of typed content blocks (text / image / embedded resource). We
    surface text blocks verbatim and JSON-encode anything else so no
    information is silently dropped.
    """
    # CallToolResult with a `.content` list of blocks.
    blocks = getattr(content, "content", None)
    if blocks is None and isinstance(content, list):
        blocks = content
    if blocks is None:
        # Not a structured result — best-effort stringification.
        if isinstance(content, str):
            return content
        try:
            return json.dumps(content, ensure_ascii=False, default=str)
        except Exception:
            return str(content)

    parts: list[str] = []
    for block in blocks:
        text = getattr(block, "text", None)
        if isinstance(text, str):
            parts.append(text)
            continue
        # Non-text block (image / resource / etc.) — serialize structurally.
        dumper = getattr(block, "model_dump", None)
        if callable(dumper):
            try:
                parts.append(
                    json.dumps(dumper(mode="json"), ensure_ascii=False, default=str)
                )
                continue
            except Exception:
                pass
        try:
            parts.append(json.dumps(block, ensure_ascii=False, default=str))
        except Exception:
            parts.append(str(block))
    return "\n".join(parts)


def _guess_image_mime_type(url: str) -> str:
    """Best-effort MIME type from a URL/path extension. Defaults to JPEG."""
    lowered = url.split("?", 1)[0].rsplit(".", 1)
    ext = lowered[-1].lower() if len(lowered) == 2 else ""
    return {
        "png": "image/png",
        "jpg": "image/jpeg",
        "jpeg": "image/jpeg",
        "gif": "image/gif",
        "webp": "image/webp",
        "heic": "image/heic",
        "heif": "image/heif",
        "bmp": "image/bmp",
    }.get(ext, "image/jpeg")


# ---------------------------------------------------------------------------
# Message → genai contents conversion
# ---------------------------------------------------------------------------
# `params.messages` is the full conversation. Each item is one of:
#
#   {"role":"user","content":<str> | [{"type":"text","text":..}
#                                     | {"type":"image","url":..}]}
#   {"role":"model","content":[{"type":"text","text":..}],
#    "tool_calls":[{"id":..,"name":..,"args":{..}}]}
#   {"role":"tool","tool_call_id":..,"name":..,"content":"<text>",
#    "is_error":bool}
#   {"role":"system","content":<str> | [...]}   # folded into system_instruction
#
# Conversion rules (mirrors STEP 3 of the runner spec):
#   user   → Content(role="user",  parts=[text / image parts])
#   model  → Content(role="model", parts=[text parts, function_call parts])
#   tool   → Content(role="user",  parts=[function_response part])
#   system → text accumulated and prepended to system_prompt
#
# Every `types.*` construction that touches an optional/version-specific
# field is guarded so a differing SDK version degrades gracefully instead
# of crashing the whole run.


def _content_to_text(content: Any) -> str:
    """Flatten a message ``content`` (string or part-list) to plain text.

    Used for system messages, which fold into ``system_instruction`` as a
    single string. Image parts are noted by their URL.
    """
    if content is None:
        return ""
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        chunks: list[str] = []
        for part in content:
            if isinstance(part, str):
                chunks.append(part)
            elif isinstance(part, dict):
                t = part.get("type")
                if t == "text":
                    chunks.append(str(part.get("text", "")))
                elif t == "image":
                    url = part.get("url")
                    if url:
                        chunks.append(f"[image: {url}]")
        return "\n".join(c for c in chunks if c)
    return str(content)


def _user_parts_from_content(content: Any) -> list[Any]:
    """Build a list of ``types.Part`` for a user message ``content`` value.

    For images we attempt ``Part.from_uri`` (a remote/GCS file reference);
    if that fails for any reason we fall back to a text part carrying the
    URL so the model still sees the reference instead of crashing the run.
    """
    parts: list[Any] = []

    if content is None:
        return parts

    if isinstance(content, str):
        if content:
            parts.append(types.Part(text=content))
        return parts

    if not isinstance(content, list):
        # Unknown shape — stringify so nothing is silently dropped.
        parts.append(types.Part(text=str(content)))
        return parts

    for part in content:
        if isinstance(part, str):
            if part:
                parts.append(types.Part(text=part))
            continue
        if not isinstance(part, dict):
            continue
        t = part.get("type")
        if t == "text":
            parts.append(types.Part(text=str(part.get("text", ""))))
        elif t == "image":
            url = part.get("url")
            if not url:
                continue
            mime_type = part.get("mime_type") or _guess_image_mime_type(str(url))
            appended = False
            try:
                parts.append(
                    types.Part.from_uri(file_uri=str(url), mime_type=mime_type)
                )
                appended = True
            except Exception:
                appended = False
            if not appended:
                # Fall back to a text part noting the URL — the model still
                # sees the reference rather than the run aborting.
                parts.append(types.Part(text=f"[image: {url}]"))
        # Unknown part types are ignored.
    return parts


def _model_parts_from_message(msg: dict[str, Any]) -> list[Any]:
    """Build ``types.Part`` list for a ``role:"model"`` message.

    Text parts first, then one ``function_call`` part per ``tool_calls``
    entry (preserving the call ``id`` so tool results can be matched).
    """
    parts: list[Any] = []

    content = msg.get("content")
    if isinstance(content, str):
        if content:
            parts.append(types.Part(text=content))
    elif isinstance(content, list):
        for part in content:
            if isinstance(part, dict) and part.get("type") == "text":
                parts.append(types.Part(text=str(part.get("text", ""))))
            elif isinstance(part, str) and part:
                parts.append(types.Part(text=part))

    for call in msg.get("tool_calls") or []:
        if not isinstance(call, dict):
            continue
        name = call.get("name")
        if not name:
            continue
        args = call.get("args")
        if not isinstance(args, dict):
            args = {}
        call_id = call.get("id")
        fc_kwargs: dict[str, Any] = {"name": str(name), "args": args}
        if call_id is not None:
            fc_kwargs["id"] = str(call_id)
        try:
            function_call = types.FunctionCall(**fc_kwargs)
        except Exception:
            # SDK may not accept `id` — retry without it.
            function_call = types.FunctionCall(name=str(name), args=args)
        parts.append(types.Part(function_call=function_call))

    return parts


def _function_response_part(
    name: str,
    content: str,
    is_error: bool,
    call_id: Optional[str],
) -> Any:
    """Build a ``types.Part`` carrying a function (tool) response.

    The response payload is ``{"error": content}`` when ``is_error`` is set,
    otherwise ``{"result": content}``. The originating call ``id`` is
    threaded onto ``FunctionResponse`` so multi-call turns stay matched —
    falling back to the ``Part.from_function_response`` factory (which has
    no ``id`` knob) if direct construction with ``id`` is unsupported by
    the installed SDK.
    """
    response: dict[str, Any] = {"error": content} if is_error else {"result": content}
    if call_id is not None:
        try:
            fr = types.FunctionResponse(name=name, response=response, id=str(call_id))
            return types.Part(function_response=fr)
        except Exception:
            pass
    try:
        return types.Part.from_function_response(name=name, response=response)
    except Exception:
        fr = types.FunctionResponse(name=name, response=response)
        return types.Part(function_response=fr)


def _build_contents(
    messages: list[Any],
    system_prompt: Optional[str],
) -> tuple[list[Any], Optional[str]]:
    """Translate ``params.messages`` into ``(contents, system_instruction)``.

    System messages are accumulated and prepended to ``system_prompt`` to
    form the combined ``system_instruction`` returned to the caller.
    """
    contents: list[Any] = []
    system_chunks: list[str] = []
    if system_prompt:
        system_chunks.append(system_prompt)

    for idx, msg in enumerate(messages):
        if not isinstance(msg, dict):
            raise ValueError(f"messages[{idx}] must be an object")
        role = msg.get("role")
        if role == "system":
            text = _content_to_text(msg.get("content"))
            if text:
                system_chunks.append(text)
        elif role == "user":
            parts = _user_parts_from_content(msg.get("content"))
            if parts:
                contents.append(types.Content(role="user", parts=parts))
        elif role == "model":
            parts = _model_parts_from_message(msg)
            if parts:
                contents.append(types.Content(role="model", parts=parts))
        elif role == "tool":
            name = msg.get("name") or ""
            call_id = msg.get("tool_call_id")
            content = msg.get("content")
            content_str = content if isinstance(content, str) else _content_to_text(content)
            is_error = bool(msg.get("is_error"))
            part = _function_response_part(
                str(name), content_str, is_error, call_id
            )
            # Function responses are delivered back to the model on the
            # "user" turn, per the Gemini function-calling protocol.
            contents.append(types.Content(role="user", parts=[part]))
        else:
            raise ValueError(f"messages[{idx}] has unknown role: {role!r}")

    system_instruction = "\n\n".join(c for c in system_chunks if c) or None
    return contents, system_instruction


# ---------------------------------------------------------------------------
# MCP wiring
# ---------------------------------------------------------------------------
# Each `mcp_servers` entry is connected over streamable HTTP; its tool
# list is turned into `types.FunctionDeclaration`s the model can call. A
# `name -> session` map routes each `tool_use` to the owning session for
# dispatch. The whole set of sessions is managed by one `AsyncExitStack`
# for the lifetime of the run.


def _function_declaration_from_tool(tool: Any) -> Optional[Any]:
    """Build a ``types.FunctionDeclaration`` from one MCP tool.

    Prefers the raw-JSON-schema knob (``parameters_json_schema``) so the
    MCP ``inputSchema`` dict is passed through unmodified; falls back to
    the typed ``parameters`` / a name+description-only declaration when an
    older SDK lacks that field.
    """
    name = getattr(tool, "name", None)
    if not name:
        return None
    description = getattr(tool, "description", None) or ""
    input_schema = getattr(tool, "inputSchema", None)
    if input_schema is None:
        input_schema = getattr(tool, "input_schema", None)

    if isinstance(input_schema, dict) and input_schema:
        # 1) Preferred: pass the raw JSON schema straight through.
        try:
            return types.FunctionDeclaration(
                name=str(name),
                description=str(description),
                parameters_json_schema=input_schema,
            )
        except Exception:
            pass
        # 2) Fallback: let the SDK coerce the dict into a typed Schema.
        try:
            return types.FunctionDeclaration(
                name=str(name),
                description=str(description),
                parameters=input_schema,
            )
        except Exception:
            pass

    # 3) Last resort: declaration with no parameters.
    try:
        return types.FunctionDeclaration(
            name=str(name), description=str(description)
        )
    except Exception:
        return None


async def _connect_mcp_servers(
    mcp_servers: dict[str, Any],
    stack: AsyncExitStack,
) -> tuple[dict[str, Any], list[Any]]:
    """Connect each MCP server and collect ``(name -> session, declarations)``.

    Every configured server is treated as required: a connection or
    ``list_tools`` failure raises (the agent definition explicitly opted
    into the server, so silent degradation would be a correctness bug).
    """
    tool_to_session: dict[str, Any] = {}
    declarations: list[Any] = []

    for name, cfg in mcp_servers.items():
        if not isinstance(cfg, dict):
            raise ValueError(f"mcp_servers[{name!r}] must be an object")
        url = cfg.get("url")
        if not isinstance(url, str) or not url:
            raise ValueError(
                f"mcp_servers[{name!r}].url must be a non-empty string"
            )
        headers = cfg.get("headers")
        if headers is not None and not isinstance(headers, dict):
            raise ValueError(
                f"mcp_servers[{name!r}].headers must be an object"
            )

        # streamablehttp_client yields (read, write, get_session_id); we
        # only need the first two for the ClientSession.
        transport = await stack.enter_async_context(
            streamablehttp_client(url, headers=headers or None)
        )
        read_stream, write_stream = transport[0], transport[1]
        session = await stack.enter_async_context(
            ClientSession(read_stream, write_stream)
        )
        await session.initialize()
        tools_result = await session.list_tools()

        for tool in tools_result.tools:
            decl = _function_declaration_from_tool(tool)
            if decl is None:
                continue
            tool_name = getattr(tool, "name", None)
            if not tool_name:
                continue
            tool_to_session[str(tool_name)] = session
            declarations.append(decl)

    return tool_to_session, declarations


# ---------------------------------------------------------------------------
# Config assembly
# ---------------------------------------------------------------------------


_EFFORT_BUDGET = {"low": 1024, "medium": 8192, "high": 24576}


def _thinking_budget(effort: Optional[str], thinking: bool) -> int:
    """Map ``effort`` to a thinking-token budget (0 disables thinking)."""
    if not thinking:
        return 0
    if isinstance(effort, str):
        budget = _EFFORT_BUDGET.get(effort.lower())
        if budget is not None:
            return budget
    return _EFFORT_BUDGET["medium"]


def _build_config(
    system_instruction: Optional[str],
    declarations: list[Any],
    web_search_enabled: bool,
    effort: Optional[str],
    thinking: bool,
) -> Any:
    """Assemble a ``types.GenerateContentConfig`` from the run params.

    Every ``types.*`` sub-object is constructed defensively: a missing /
    renamed field in the installed SDK is skipped rather than aborting the
    whole run. The config is then built from only the keys that were
    successfully populated.
    """
    config_kwargs: dict[str, Any] = {}

    if system_instruction:
        config_kwargs["system_instruction"] = system_instruction

    # Tools: MCP function declarations + optional Google Search grounding.
    tools: list[Any] = []
    if declarations:
        try:
            tools.append(types.Tool(function_declarations=declarations))
        except Exception:
            pass
    if web_search_enabled:
        try:
            tools.append(types.Tool(google_search=types.GoogleSearch()))
        except Exception:
            pass
    if tools:
        config_kwargs["tools"] = tools

    # Disable the SDK's built-in automatic function-calling loop: this
    # runner drives the agentic loop itself so it can stream every step
    # and dispatch tool calls to MCP sessions.
    try:
        config_kwargs["automatic_function_calling"] = (
            types.AutomaticFunctionCallingConfig(disable=True)
        )
    except Exception:
        pass

    # Thinking config — include thoughts so we can stream them, with a
    # budget mapped from `effort` (0 when thinking is disabled).
    try:
        budget = _thinking_budget(effort, thinking)
        config_kwargs["thinking_config"] = types.ThinkingConfig(
            include_thoughts=True,
            thinking_budget=budget,
        )
    except Exception:
        pass

    try:
        return types.GenerateContentConfig(**config_kwargs)
    except Exception:
        # If a key is rejected outright, retry with progressively fewer
        # optional knobs so the run can still proceed.
        for drop in ("thinking_config", "automatic_function_calling", "tools"):
            config_kwargs.pop(drop, None)
            try:
                return types.GenerateContentConfig(**config_kwargs)
            except Exception:
                continue
        return types.GenerateContentConfig()


# ---------------------------------------------------------------------------
# Per-request handler
# ---------------------------------------------------------------------------


# Safety cap on the agentic tool-calling loop. Each iteration is one
# model turn; tool results feed the next turn. A diag is emitted if hit.
_MAX_ITERATIONS = 25


async def handle_run(
    request_id: str,
    params: dict[str, Any],
    stdout_writer: Writer,
    stderr_writer: Writer,
) -> None:
    """Run one Gemini SDK conversation, tagging every emit with
    ``request_id``. Always emits exactly one terminal ``end`` line via
    ``asyncio.shield`` even on cancellation.

    The Rust caller enforces the FIFO concurrency cap on its side, so
    every ``run`` that reaches this function already holds a slot —
    there is nothing to wait for here.
    """
    status: str = "ok"
    error: Optional[str] = None

    # Propagate the composite agent id into this process's environment so
    # any objectiveai cli invocation an MCP server makes downstream sees
    # OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY. Unlike the codex/claude
    # runners (which route env through an SDK-spawned subprocess), the
    # genai client runs in-process, so we set it on os.environ.
    agent_instance_hierarchy = params.get("agent_instance_hierarchy")
    if agent_instance_hierarchy:
        os.environ["OBJECTIVEAI_AGENT_INSTANCE_HIERARCHY"] = str(
            agent_instance_hierarchy
        )

    try:
        model: str = params["model"]
        messages: list[Any] = params["messages"]
        system_prompt = params.get("system_prompt")
        effort = params.get("effort")
        thinking = params.get("thinking", True)
        web_search_enabled = bool(params.get("web_search_enabled"))
        mcp_servers: dict[str, Any] = params.get("mcp_servers") or {}

        # `genai.Client()` reads GEMINI_API_KEY / GOOGLE_API_KEY from env.
        client = genai.Client()

        contents, system_instruction = _build_contents(messages, system_prompt)

        async with AsyncExitStack() as stack:
            tool_to_session, declarations = await _connect_mcp_servers(
                mcp_servers, stack
            )

            config = _build_config(
                system_instruction,
                declarations,
                web_search_enabled,
                effort,
                thinking,
            )

            last_usage: Any = None

            for iteration in range(_MAX_ITERATIONS):
                # Parts accumulated for the assistant turn we append back
                # into `contents` so the model sees its own prior output.
                assistant_parts: list[Any] = []
                pending_calls: list[Any] = []

                stream = await client.aio.models.generate_content_stream(
                    model=model,
                    contents=contents,
                    config=config,
                )

                async for chunk in stream:
                    usage = getattr(chunk, "usage_metadata", None)
                    if usage is not None:
                        last_usage = usage

                    candidates = getattr(chunk, "candidates", None)
                    if not candidates:
                        continue
                    candidate = candidates[0]
                    content = getattr(candidate, "content", None)
                    if content is None:
                        continue
                    parts = getattr(content, "parts", None) or []

                    for part in parts:
                        assistant_parts.append(part)

                        function_call = getattr(part, "function_call", None)
                        if function_call is not None:
                            pending_calls.append(function_call)
                            continue

                        text = getattr(part, "text", None)
                        if text:
                            is_thought = bool(getattr(part, "thought", False))
                            kind = "thinking" if is_thought else "text"
                            await stdout_writer.emit_event(
                                request_id, {"kind": kind, "text": text}
                            )

                # Record the assistant turn (text + any function-call parts)
                # so the next request carries the full history.
                if assistant_parts:
                    contents.append(
                        types.Content(role="model", parts=assistant_parts)
                    )

                if not pending_calls:
                    break

                # Dispatch every function call this turn produced, then loop.
                tool_response_parts: list[Any] = []
                for call in pending_calls:
                    name = getattr(call, "name", None) or ""
                    args = getattr(call, "args", None)
                    if not isinstance(args, dict):
                        args = {}
                    call_id = getattr(call, "id", None)

                    await stdout_writer.emit_event(
                        request_id,
                        {
                            "kind": "tool_use",
                            "id": call_id,
                            "name": name,
                            "input": args,
                        },
                    )

                    session = tool_to_session.get(name)
                    if session is None:
                        result_text = f"No MCP tool registered named {name!r}"
                        is_error = True
                    else:
                        try:
                            raw_result = await session.call_tool(name, args)
                            result_text = _stringify_tool_content(raw_result)
                            is_error = bool(getattr(raw_result, "isError", False))
                        except Exception as e:
                            result_text = _one_line(
                                str(e) or e.__class__.__name__
                            )
                            is_error = True

                    await stdout_writer.emit_event(
                        request_id,
                        {
                            "kind": "tool_result",
                            "tool_use_id": call_id,
                            "content": result_text,
                            "is_error": is_error,
                        },
                    )

                    tool_response_parts.append(
                        _function_response_part(
                            name, result_text, is_error, call_id
                        )
                    )

                contents.append(
                    types.Content(role="user", parts=tool_response_parts)
                )
            else:
                # Loop fell through without `break` → hit the safety cap.
                await stderr_writer.emit_diag(
                    request_id,
                    "warn",
                    f"agentic loop hit max iterations ({_MAX_ITERATIONS})",
                )

            # Emit the single usage event mapped from the last chunk that
            # carried usage_metadata (None fields counted as 0).
            if last_usage is not None:
                await stdout_writer.emit_event(
                    request_id,
                    {
                        "kind": "usage",
                        "input_tokens": _coerce_int(
                            getattr(last_usage, "prompt_token_count", None)
                        ),
                        "output_tokens": _coerce_int(
                            getattr(last_usage, "candidates_token_count", None)
                        ),
                        "thinking_tokens": _coerce_int(
                            getattr(last_usage, "thoughts_token_count", None)
                        ),
                        "total_tokens": _coerce_int(
                            getattr(last_usage, "total_token_count", None)
                        ),
                    },
                )
    except asyncio.CancelledError:
        # Reachable only on subprocess SIGTERM during the drain phase.
        # In-flight cancellation isn't part of the wire protocol — surface
        # the abort as an error so the consumer sees the work didn't
        # finish. Re-raised in the finally after the terminal end line.
        status = "error"
        error = "cancelled"
        raise
    except Exception as e:
        status = "error"
        error = _one_line(str(e) or e.__class__.__name__)
    finally:
        # Shield the terminal emit so that a second cancel arriving while
        # the SDK / MCP sessions are unwinding doesn't suppress it.
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


_REQUIRED_PARAMS = ("model", "messages")


def _validate_run_params(params: Any) -> Optional[str]:
    """Return None if params is acceptable, else a one-line error string."""
    if not isinstance(params, dict):
        return "params must be an object"
    for key in _REQUIRED_PARAMS:
        if key not in params:
            return f"missing required field '{key}'"
    if not isinstance(params["model"], str) or not params["model"]:
        return "'model' must be a non-empty string"
    if not isinstance(params["messages"], list):
        return "'messages' must be an array"
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
