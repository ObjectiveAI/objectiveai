# Subagent correlation: multiple delegations in one turn

From the pinned source (`sources/hermes-agent`, v0.20.6). How the
`/v1/runs` stream orders `tool.started` / `tool.completed` for
`delegate_task` calls against the `subagent.start` /
`subagent.complete` events of their children, when one assistant
turn issues several delegations — and therefore how a consumer
attributes each child to the call that spawned it. The ordering
is not documented anywhere; the code fixes it.

## The top level is strictly sequential

`delegate_task` is on no parallel allowlist
(`agent/tool_dispatch_helpers.py:48-61`), so it is a barrier
(`:268`), and barriers coalesce into one sequential segment
(`:196-200`, `:271-278`). A turn of
`[web_search, delegate_task(2 tasks), delegate_task(1 task),
terminal]` plans as ONE sequential segment (the lone `web_search`
run has length 1 and is demoted) and runs on one thread in emission
order (`run_agent.py:8449-8462`). So the delegate calls' own events
never interleave or nest:
`started(d1) … completed(d1), started(d2) … completed(d2)`.

Children can never corrupt that pairing: a child's
`tool.started` is renamed `subagent.tool` and its `tool.completed`
is dropped before anything reaches the parent's callback
(`_LEGACY_EVENT_MAP`, `tools/delegate_tool.py:1159-1165`; drop at
`:1500-1501`, relay at `:1549`), and the gateway discards
`subagent.tool` anyway (`api_server.py:7587-7590`). EVERY
`tool.started`/`tool.completed` on `/v1/runs` is the top-level
agent's — the FIFO of `parallel-tool-calls.md` stands.

## On /v1/runs, delegation is ALWAYS background

`delegate_task` runs its children synchronously only when it has
no session to wake later; the API server declares itself a
non-delivering channel (`_bind_api_server_session` hardwires
`async_delivery=False`, `api_server.py:7186-7215`), BUT
`delegate_task` then checks for a bound session id and, finding
one, dispatches in the background regardless and wakes the session
by self-POSTing `/v1/chat/completions` when the batch completes
(`tools/delegate_tool.py:4139-4182`; origin id via
`_current_origin_session_id`, `tools/async_delegation.py`). And
`/v1/runs` ALWAYS binds one: `session_id = session_id or run_id`
(`api_server.py:~7681`), bound as `chat_id` precisely so
"background delegations [do not] stay forced-sync"
(`api_server.py:7814-7822`). There is no config knob for forcing
synchronous delegation (none in `async_delegation.py`; the
`background` argument is accepted and ignored,
`delegate_tool.py:4845-4847`). Only a child's own delegation (depth
> 0) is synchronous (`run_agent.py:8493-8500`).

The consequences of background dispatch, all code-fixed:

- Children are CONSTRUCTED on the tool thread (`:3870-3930`) — that
  emits only `subagent.spawn_requested`, which is dropped
  (`:2073-2077` → `:1479-1487`) — then the whole batch is handed
  to a shared executor with NO latch, no future awaited, no
  "started" barrier (`tools/async_delegation.py:1135-1158`), and
  the tool returns `{"status": "dispatched", …}` (`:4307-4349`).
- `subagent.start` is emitted from each child's WORKER THREAD, in
  `_run_single_child` (`:2699-2703`). It therefore RACES
  `tool.completed(d)`: it lands on either side, usually before
  (the tool has JSON to build and a result to persist first), but
  by timing, not by rule. It can also land after `tool.started(d2)`
  — batches run concurrently on the shared pool
  (`async_delegation.py:600-616`).
- Siblings are submitted in `task_index` order but run on
  `max_children` workers (`:3963-3979`): `start(task 1)` may
  precede `start(task 0)`.
- `subagent.complete` (`:3279-3282`) is arbitrarily later —
  routinely after later tools, and AFTER THE RUN ENDS. The run's
  sentinel tears the queue down (`api_server.py:7991-7995`,
  `:8081`), after which `_push` finds no queue and discards
  silently (`:7508-7510`). Background completions are usually
  LOST to `/v1/runs`.
- Stream order is otherwise honest: `_push` goes through
  `loop.call_soon_threadsafe(q.put_nowait, event)` into one queue
  with one consumer (`:7508-7514`, `:8065-8078`), so SSE order is
  the real-time order of callback invocations across all threads.
  Nothing batches or delays `subagent.start`/`complete` (the
  `_BATCH_SIZE = 5` rollup at `:1405`, `:1549-1555` concerns only
  the dropped `subagent.tool` path).

The timeline for the example turn, background mode:

```
1 tool.started    web_search
2 tool.completed  web_search
3 tool.started    delegate_task   preview "2 tasks: g0[:37]… | g1[:37]…"
4 tool.completed  delegate_task   ← race boundary
5 tool.started    delegate_task   preview "1 tasks: g2[:37]…"
6 tool.completed  delegate_task   ← race boundary
7 tool.started    terminal
8 tool.completed  terminal
  run.completed, sentinel, stream closed
— unordered against 4-8 and among themselves —
  subagent.start (task 0/2, sid sa-0-…)  ‖  (task 1/2, sa-1-…)
  ‖ subagent.start (task 0/1, sa-0-…)
  ‖ subagent.complete(…)   ← typically after close ⇒ dropped
```

What IS guaranteed: `subagent.start(A*)` comes after
`tool.started(d1)` (the submit is inside the tool body). What is
NOT: before `tool.completed(d1)`, before `tool.started(d2)`, or in
sibling order.

## Attribution: by content, not by position

`parent_id` is the parent AGENT's subagent id — absent for every
child of the top-level agent (`:1658-1660`, omitted when None at
`:1417-1418`) — and `subagent_id` (`sa-<task_index>-<8 hex>`)
embeds the task index, not the call. So neither names the spawning
call. What does is the GOAL:

- `subagent.start` carries `goal` — the full, stripped goal
  (`:1403`, `:1417`), plus `task_index` and `task_count`.
- `tool.started(delegate_task)` carries a `preview` built from ALL
  the call's goals: `"<N> tasks: <g0> | <g1> | …"`, each goal
  one-lined and truncated to 40 chars as a 37-char prefix plus
  `"..."` (`agent/display.py:478-497`, `:417-427`, `:183-188`;
  outer cap unlimited by default, `:113`). A scalar `goal=` call
  yields the full one-lined goal.

The rule:

1. On `tool.started(delegate_task)`, parse the preview into
   `(task_count, [fragments])` — split on `" tasks: "`, then
   `" | "`. Record the call as an open delegation.
2. On `subagent.start`, match the open delegation whose
   `task_count` equals the event's and whose fragment at
   `task_index` equals the event's `goal`, or is a prefix of it
   with the trailing `"..."` removed (exact for goals ≤ 37 chars,
   prefix beyond). Consume that slot.
3. Where two open delegations are indistinguishable (same
   `task_count`, identical fragments), the earlier `tool.started`
   is the conventional owner of the first-arriving start — but
   FLAG it as ambiguous rather than assert it; the code offers no
   tiebreak.
4. From there, key everything by `subagent_id`: `subagent.complete`
   carries the same id (`run-event-stream.md`), so a completion is
   unambiguous whenever it arrives — if it arrives.
5. Depth > 0 events belong to the `subagent_id` in their
   `parent_id`, never to a top-level call: a child's delegation is
   synchronous, its grandchildren carry `depth = 1`, `parent_id` =
   the child's `sa-…` id, and their own fresh `subagent_id`
   (`:1648-1660`, `:2026-2033`); the child's relay re-relays them
   with their own identity intact (`:1437-1438`, `:1450-1460`).
6. Never wait on `/v1/runs` for a background `subagent.complete`.

## Open for the harness

Background dispatch means a run can END with children still
running, and their results then wake the SESSION by a self-POSTed
`/v1/chat/completions` turn — a turn our driver did not start,
whose events do not reach our run's stream, and whose tool calls
would still transit our proxy. Nothing in the request or config
forces synchronous delegation on `/v1/runs`. The choices are the
harness's: leave `delegation` off unless asked (its only
vocabulary is the caller's `Option<bool>`), accept the detached
turn and study the wake path, or something cleverer. To be settled
in `HARNESS.md` when the run driver is designed.
