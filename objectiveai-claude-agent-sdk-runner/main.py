#!/usr/bin/env python3
"""ObjectiveAI Claude Agent SDK Runner.

Runs the Claude Agent SDK and streams JSONL events to stdout.
Designed to be spawned as a subprocess by objectiveai-api, replacing
the inline-generated Node.js script with a standalone Python program.

All parameters that were previously baked into the generated JS code
are accepted as CLI arguments instead.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import sys
from typing import Any

from claude_agent_sdk import query, ClaudeAgentOptions
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
# CLI argument parsing
# ---------------------------------------------------------------------------


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run Claude Agent SDK and stream JSONL events to stdout.",
    )
    parser.add_argument(
        "--model",
        required=True,
        help="Model identifier (e.g. claude-sonnet-4-20250514)",
    )
    parser.add_argument(
        "--message",
        required=True,
        help="SDK user message as a JSON string",
    )
    parser.add_argument(
        "--system-prompt",
        default=None,
        help="System prompt text",
    )
    parser.add_argument(
        "--effort",
        choices=["low", "medium", "high", "max"],
        default=None,
        help="Effort level for thinking depth",
    )
    parser.add_argument(
        "--thinking-disabled",
        action="store_true",
        help="Disable extended thinking",
    )
    parser.add_argument(
        "--mcp-servers",
        default=None,
        help='MCP servers config as a JSON object (e.g. \'{"name": {"type": "http", "url": "..."}}\')',
    )
    parser.add_argument(
        "--resume",
        default=None,
        help="Session ID to resume",
    )
    parser.add_argument(
        "--user-agent",
        default=None,
        help="User agent string (sets CLAUDE_AGENT_SDK_CLIENT_APP env var)",
    )
    return parser.parse_args()


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


async def run(args: argparse.Namespace) -> None:
    # Parse message JSON.
    message: dict[str, Any] = json.loads(args.message)

    # Parse MCP servers config.
    mcp_servers: dict[str, Any] = {}
    if args.mcp_servers:
        mcp_servers = json.loads(args.mcp_servers)

    # Build thinking config.
    thinking = None
    if args.thinking_disabled:
        thinking = {"type": "disabled"}

    # Build env overrides.
    env: dict[str, str] = {}
    if args.user_agent:
        env["CLAUDE_AGENT_SDK_CLIENT_APP"] = args.user_agent

    # Build options — mirrors the JS opts from options.rs.
    opts = ClaudeAgentOptions(
        model=args.model,
        system_prompt=args.system_prompt,
        effort=args.effort,
        thinking=thinking,
        mcp_servers=mcp_servers,
        resume=args.resume,
        env=env,
        # Matches JS: tools: []
        tools=[],
        # Matches JS: includePartialMessages: true
        include_partial_messages=True,
        # Matches JS: permissionMode: "bypassPermissions"
        permission_mode="bypassPermissions",
    )

    # Create an async generator that yields the single SDK user message,
    # mirroring the JS: async function* messages() { yield message; }
    async def messages():
        yield message

    # Stream events as JSONL to stdout.
    async for msg in query(prompt=messages(), options=opts):
        d = serialize_message(msg)
        if d is not None:
            sys.stdout.write(json.dumps(d) + "\n")
            sys.stdout.flush()


def main() -> None:
    # Remove CLAUDECODE env var to avoid conflicts with the SDK subprocess.
    # Matches JS: delete process.env.CLAUDECODE;
    os.environ.pop("CLAUDECODE", None)

    args = parse_args()
    try:
        asyncio.run(run(args))
    except Exception as e:
        sys.stderr.write(str(e))
        sys.exit(1)


if __name__ == "__main__":
    main()
