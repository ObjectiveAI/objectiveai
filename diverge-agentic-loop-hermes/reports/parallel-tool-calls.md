# Parallel tool calls and the order of their events

From the pinned source (`sources/hermes-agent`, v0.20.6). Whether
Hermes runs tool calls in parallel, how its `tool.started` /
`tool.completed` events come out when it does, and what that means
for a consumer that wants to pair them into synthetic tool-call
ids from stream order alone — since the gateway stream carries no
ids of its own (`run-event-stream.md`).

## Yes, it runs them in parallel — by allowlist, not by switch

The decision is per batch, structural, and unconfigurable
(`run_agent.py:8428-8470`, planner
`agent/tool_dispatch_helpers.py:149-280`):

- A single call always goes sequential; ≥2 calls are required,
  and any would-be parallel run of fewer than 2 is demoted
  (`tool_dispatch_helpers.py:274-275`).
- Only allowlisted tools are parallel-eligible
  (`tool_dispatch_helpers.py:48-61`): `web_search`,
  `web_extract`, `read_file`, `search_files`, `vision_analyze`,
  `image_generate`, `session_search`, `skill_view`,
  `skills_list`, a few Home Assistant reads, and the read-only
  bridge lookups. EVERYTHING ELSE is a barrier — `terminal`,
  `patch`, `write_file` with conflicts, `delegate_task`, memory.
  `clarify` is hard never-parallel (`:45`). Unparseable args are
  a barrier (`:208-226`); reader/writer path overlap closes a
  parallel run (`:237-258`).
- MCP tools are barriers UNLESS their server's config sets
  `supports_parallel_tool_calls: true` — default false
  (`tools/mcp_tool.py:7698`, gate `:7939-7954`). So our proxy's
  tools are sequential until the harness opts them in.
- Mixed batches run as segments in strict model order
  (`execute_tool_calls_segmented`, `tool_executor.py:2867-2924`).
- Parallelism is capped at 8 OS threads, hardcoded
  (`_MAX_TOOL_WORKERS`, `tool_executor.py:124`), with
  `image_gen.max_parallel_requests` (default 4) the one tunable
  sub-cap (`:242-266`). Batch deadline
  `timeouts.tools.concurrent_batch`, default 420s (`:195-208`).

## The events are serialized in original call order — never interleaved

Execution is concurrent; emission is not.

- Every `tool.started` passes through a start-order gate
  (`_begin_in_order`, `tool_executor.py:1251-1296`): worker k+1
  cannot fire its start until worker k's has returned. Starts
  come out in the model's original call order.
- Every `tool.completed` is emitted only AFTER the whole batch has
  finished, in a post-execution loop over `parsed_calls` — again
  original order, regardless of finish order
  (`tool_executor.py:1701-1703`, callback `:1863-1869`).

So a healthy concurrent batch of N reads as
`started(0) … started(N-1), completed(0) … completed(N-1)`. A
batch where B finishes first still emits `completed(A)` before
`completed(B)`. **Pairing the i-th started with the i-th completed
is correct**, and that is the synthetic id: a counter.

Same-name duplicates (two `web_search` calls in one batch) are
legal and only distinguishable by ordinal; `tool.started` carries
an args-derived `preview` that labels them, `tool.completed`
carries nothing but name, duration and the error flag
(`:1866` — positions 3 and 4 are hardcoded `None`).

## No id was dropped: none was ever sent

The executor has `tool_call_id` in scope at both emission sites
but passes it only to a DIFFERENT callback pair —
`tool_start_callback(tool_call_id, name, args)` (`:1067-1069`) and
`tool_complete_callback(tool_call_id, name, args, result)`
(`:1891-1893`, `:2796-2801`) — which `/v1/runs` leaves unwired
(`api_server.py:2820-2821`, `:7757`). The wired
`tool_progress_callback` receives `(event, name, preview, args)`
plus `duration`/`is_error`/`result` kwargs and no id
(`:1048-1057`, `:1865-1869`, `:2782-2786`). The omission is
deliberate — `tool.output_risk` on the same callback DOES pass
`tool_call_id=` (`:1908`, `:2816`).

## What breaks the counter, and how to notice

- A guardrail-blocked call emits NEITHER event (`:682`, `:1863`)
  — symmetric; alignment survives.
- A call abandoned at the gate or timed out can emit `completed`
  with NO `started` (`:1368-1371`, `:1406-1416`, `:1714-1762`) —
  asymmetric. A completion with no open start of that name means
  the batch degraded.
- A mid-batch session-DB persistence failure truncates the
  completion tail (`:1853-1858`).
- The start-order gate times out after 120s (`:133`, clamped to
  half the batch deadline) and then proceeds OUT OF ORDER with a
  warning (`:1283-1289`) — only under a wedged dispatch.

## Consequence for the MCP proxy

Within one call, `tool.started` always precedes that call's proxy
request (`:714` then `:732`). Across concurrent calls there is no
ordering guarantee: proxy arrival order and completion-event order
are unrelated, and correlation degrades to (name, ordinal within
the batch). Leaving `supports_parallel_tool_calls` at its default
keeps every proxy call a sequential barrier — proxy order then
matches stream order, and the enqueue fold's "next tool response"
stays a well-defined next. That is the harness's decision to make;
the cost is parallelism on the caller's tools only.
