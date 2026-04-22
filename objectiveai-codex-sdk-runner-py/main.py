#!/usr/bin/env python3
"""ObjectiveAI Codex SDK Runner.

Runs the official OpenAI Codex Python SDK (``openai-codex-sdk``) and streams
thread events to stdout as NDJSON. Designed to be spawned as a subprocess by
``objectiveai-api``.

Authentication is inherited from the user's ``~/.codex/auth.json`` (written
by ``codex login``). The SDK shells out to the ``codex`` binary which reads
that file — we do nothing special for auth.

Codex binary resolution order:
  1. ``--codex-bin`` CLI arg (maps to ``CodexOptions.codex_path_override``)
  2. ``CODEX_BIN`` environment variable
  3. ``shutil.which("codex")`` (system PATH)
  4. SDK default resolution (whatever ``CodexExec`` does with None)
"""

from __future__ import annotations

__version__ = "2.0.0"

import argparse
import asyncio
import json
import os
import shutil
import sys
from typing import Any

from openai_codex_sdk import (
    Codex,
    LocalImageInput,
    TextInput,
)


# ---------------------------------------------------------------------------
# Input parsing
# ---------------------------------------------------------------------------
# ``--input`` accepts a JSON array of input items. Each item is one of:
#   {"type": "text",        "text": "..."}
#   {"type": "local_image", "path": "..."}
# A plain string may also be passed for convenience — it becomes a single
# TextInput.


def _parse_input(raw: Any) -> Any:
    if isinstance(raw, str):
        return raw
    if not isinstance(raw, list):
        raise ValueError("--input must be a JSON array of input items or a plain string")

    items: list[Any] = []
    for idx, item in enumerate(raw):
        if not isinstance(item, dict) or "type" not in item:
            raise ValueError(f"--input[{idx}] must be an object with a 'type' field")
        t = item["type"]
        try:
            if t == "text":
                items.append(TextInput(type="text", text=item["text"]))
            elif t == "local_image":
                items.append(LocalImageInput(type="local_image", path=item["path"]))
            else:
                raise ValueError(f"--input[{idx}] has unknown type: {t!r}")
        except KeyError as e:
            raise ValueError(f"--input[{idx}] missing required field: {e.args[0]}")
    return items


# ---------------------------------------------------------------------------
# Event serialization
# ---------------------------------------------------------------------------
# Each ThreadEvent from the SDK is a pydantic model. We emit it as NDJSON
# using the model's own dump (camelCase via alias-aware mode is not needed
# here — the SDK's wire format is already snake_case on the event layer).


def _serialize_event(event: Any) -> dict[str, Any]:
    if hasattr(event, "model_dump"):
        return event.model_dump(mode="json", by_alias=False, exclude_none=False)
    if isinstance(event, dict):
        return event
    return {"_repr": repr(event)}


def _emit(obj: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


# ---------------------------------------------------------------------------
# Codex binary resolution
# ---------------------------------------------------------------------------


def _resolve_codex_bin(explicit: str | None) -> str | None:
    """Return the path to the ``codex`` binary, or ``None`` to let the SDK decide."""
    if explicit:
        return explicit
    from_env = os.environ.get("CODEX_BIN")
    if from_env:
        return from_env
    found = shutil.which("codex")
    if found:
        return found
    return None


# ---------------------------------------------------------------------------
# CLI argument parsing
# ---------------------------------------------------------------------------


def _truthy_flag(parser: argparse.ArgumentParser, name: str, help_text: str) -> None:
    """Add a tri-state flag: --name / --no-name / absent (None)."""
    group = parser.add_mutually_exclusive_group()
    group.add_argument(
        f"--{name}",
        dest=name.replace("-", "_"),
        action="store_const",
        const=True,
        default=None,
        help=help_text,
    )
    group.add_argument(
        f"--no-{name}",
        dest=name.replace("-", "_"),
        action="store_const",
        const=False,
        help=f"Disable: {help_text}",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the OpenAI Codex Python SDK and stream events as NDJSON to stdout.",
    )
    parser.add_argument(
        "--model",
        default=None,
        help="Codex model identifier (e.g. gpt-5). Optional if --resume is used.",
    )
    parser.add_argument(
        "--input",
        required=True,
        help="Turn input: either a JSON array of input items or a plain string. "
        "Input items: {\"type\":\"text\",\"text\":...} or {\"type\":\"local_image\",\"path\":...}",
    )
    parser.add_argument(
        "--effort",
        choices=["minimal", "low", "medium", "high"],
        default=None,
        help="Model reasoning effort.",
    )
    parser.add_argument(
        "--sandbox",
        choices=["read-only", "workspace-write", "danger-full-access"],
        default=None,
        help="Sandbox mode for the codex subprocess.",
    )
    parser.add_argument(
        "--approval-policy",
        choices=["never", "on-request", "on-failure", "untrusted"],
        default="never",
        help="Approval policy. Defaults to 'never' for headless operation.",
    )
    parser.add_argument(
        "--cwd",
        default=None,
        help="Working directory for the thread (maps to ThreadOptions.working_directory).",
    )
    parser.add_argument(
        "--additional-directory",
        dest="additional_directories",
        action="append",
        default=None,
        help="Additional directory the sandbox may access. Repeat for multiple.",
    )
    parser.add_argument(
        "--output-schema",
        default=None,
        help="Structured-output JSON schema as a JSON object string.",
    )
    parser.add_argument(
        "--resume",
        default=None,
        help="Thread id to resume instead of starting a new thread.",
    )
    parser.add_argument(
        "--codex-bin",
        default=None,
        help="Path to the codex binary (overrides $CODEX_BIN and PATH lookup).",
    )
    parser.add_argument(
        "--base-url",
        default=None,
        help="Override Codex API base URL (for enterprise / custom endpoints).",
    )
    parser.add_argument(
        "--api-key",
        default=None,
        help="Override API key (bypasses ChatGPT subscription auth).",
    )
    _truthy_flag(parser, "skip-git-repo-check", "Skip the git repo check in the sandbox.")
    _truthy_flag(parser, "network-access-enabled", "Allow network access from the sandbox.")
    _truthy_flag(parser, "web-search-enabled", "Allow the agent to use web search.")
    return parser.parse_args()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def _only_set(d: dict[str, Any]) -> dict[str, Any]:
    return {k: v for k, v in d.items() if v is not None}


async def run(args: argparse.Namespace) -> int:
    input_ = _parse_input(json.loads(args.input))
    output_schema = json.loads(args.output_schema) if args.output_schema else None

    if not args.resume and not args.model:
        raise ValueError("--model is required unless --resume is given")

    codex_bin = _resolve_codex_bin(args.codex_bin)
    codex_options = _only_set({
        "codex_path_override": codex_bin,
        "base_url": args.base_url,
        "api_key": args.api_key,
    })
    codex = Codex(codex_options)

    thread_options = _only_set({
        "model": args.model,
        "sandbox_mode": args.sandbox,
        "approval_policy": args.approval_policy,
        "working_directory": args.cwd,
        "additional_directories": args.additional_directories,
        "model_reasoning_effort": args.effort,
        "skip_git_repo_check": args.skip_git_repo_check,
        "network_access_enabled": args.network_access_enabled,
        "web_search_enabled": args.web_search_enabled,
    })

    if args.resume:
        thread = codex.resume_thread(args.resume, thread_options)
    else:
        thread = codex.start_thread(thread_options)

    turn_options = _only_set({"output_schema": output_schema})
    streamed = await thread.run_streamed(input_, turn_options)

    exit_code = 0
    async for event in streamed.events:
        _emit(_serialize_event(event))
        event_type = getattr(event, "type", None)
        if event_type == "turn.failed" or event_type == "error":
            exit_code = 1

    return exit_code


def main() -> None:
    args = parse_args()
    try:
        code = asyncio.run(run(args))
    except Exception as e:  # noqa: BLE001 - surface everything to stderr
        sys.stderr.write(f"{type(e).__name__}: {e}\n")
        sys.exit(1)
    sys.exit(code)


if __name__ == "__main__":
    main()
