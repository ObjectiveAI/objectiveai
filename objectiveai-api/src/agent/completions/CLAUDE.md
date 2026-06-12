# Agent Completions

## UpstreamClient Rules

1. **No error chunk as the first item.** If the upstream would fail before producing any non-error chunk, it must return `Err(...)` from `create` instead of yielding an error chunk into the stream.

2. **No empty streams.** If the upstream produces no chunks at all, it must return `Err(...)` from `create` instead of an empty stream.

These rules apply to all `UpstreamClient` implementations (openrouter, claude_agent_sdk, mock).

## Continuation

A `Continuation` carries the conversation state between successive
`create_streaming` calls. It contains:

- **`ContinuationItem::State`** — upstream-specific state (e.g. tool call
  count for mock, session data for Claude Agent SDK).
- **`ContinuationItem::UserMessage`** — a user message appended after the
  previous assistant turn.
- **`ContinuationItem::ToolMessage`** — a tool result appended after a tool
  call.

The upstream client receives both `params.messages` (the initial fixed
messages) and the continuation items. Together they form the full
conversation history: `params.messages` is the conversation prefix that
never changes, and the continuation items are everything that happened
after.

### How `create_streaming` uses messages + continuation

1. `params.messages` are merged with any agent-specific messages (e.g.
   prefix/suffix), prepared, and optionally transformed.
2. Continuation items are passed as `&[ContinuationItem<STATE>]` to the
   upstream client's `create()`, which appends them to the prepared
   messages to reconstruct the full conversation.

### Important: `params.messages` is fixed

Once set for the first call, `params.messages` must not change across
subsequent calls within the same conversation. New user turns (step
prompts, retry prompts) go onto the continuation as `UserMessage` items.

