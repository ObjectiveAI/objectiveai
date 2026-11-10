# Delegation observability: supporting background children without disabling them

From the pinned source (`sources/hermes-agent`, v0.20.6). Whether
a harness driving `/v1/runs` can observe the OUTCOME of background
`delegate_task` children — each child's completion, the
consolidated delegation result, and the parent's follow-up turn
that consumes it — as events. Verdict: yes, through wires the
gateway already ships, config-only; but not on the originating
run's SSE stream, and not by holding the run open.

## What the run's own stream can and cannot carry

| the thing | on `/v1/runs/{id}/events`? |
|---|---|
| each child's `subagent.complete` (summary, status, output_tail, tokens, cost, child_session_id) | YES — while the parent turn is still running. The child keeps the run's event callback after detachment (`tools/delegate_tool.py:1395`, `:4226-4237`, emission `:3252-3282`; forwarded `api_server.py:7542-7585`). A child finishing after the run's sentinel is dropped (`:7508-7510`, `:7995`, `:8073-8084`). |
| the consolidated `[ASYNC DELEGATION BATCH COMPLETE …]` block | NEVER — delivered by a self-POST to `/v1/chat/completions`, a route with no run record and no stream. |
| the parent's follow-up turn consuming it | NEVER on that run's stream — it is a separate, un-instrumented chat-completion turn whose response body the waker reads and discards (`gateway/wake.py:163`). |

## The delivery path, exactly

- Batch completion → `_finalize_batch` → `_persist_completion` (the
  FULL combined results into state.db's `async_delegations` table:
  `delegation_id`, `origin_session_id` = the run's session id,
  `state`, `event_json`, `result_json`, `delivery_state`;
  `tools/async_delegation.py:158-198`, `:1226`) → a completion
  queue put (`:1226-1234`).
- A 2-second-tick watcher drains the queue and delivers
  (`gateway/run.py:26881-26943`). Its docstring says "idle case";
  its body has NO busy check — the only gate is session liveness
  (`_classify_completion_target`, `run.py:26311-26376`: deliver
  unless the session has `ended_at`). So delivery ALWAYS means a
  new turn, even mid-run.
- Delivery on the API server = `_self_post_chat_completion`
  (`gateway/wake.py:97-184`): `http://{host}:{port}/v1/chat/completions`
  — the route is a string literal (`:125`), no env, no config —
  with `Authorization: Bearer {API_SERVER_KEY}` (a missing key is
  a hard `RuntimeError`, `:116-121`) and `X-Hermes-Session-Id`, a
  plain user message carrying the formatted block
  (`tools/process_registry.py:2963-3018`), `stream: false`.
- The server side loads the session's history from state.db and
  runs an agent turn with no progress callback and no delta
  callback (`api_server.py:5042`, `:5146-5153`, `:5285-5296`);
  the turn's messages are appended to the same session row.
- HAZARD: the API server has no per-session lock — a batch
  completing while the parent turn still runs fires a CONCURRENT
  turn on the same session, last-writer-wins (`wake.py:38-42`);
  and the server's global `max_concurrent_runs` 429 cap applies
  to the self-POST too.

Dead ends, verified: no mechanism holds the run open
(`_run_and_close` sentinels unconditionally, `api_server.py:7871`,
`:7993-7997`; `agent/turn_finalizer.py` never references
delegation; children are detached from the parent's interrupt
list by design, `delegate_tool.py:4224-4240`);
`delegate_task(action="list")` returns LIVE children only, never
results (`:466-513`), and the dispatch text tells the model not to
wait or poll (`:4308-4330`); the wake cannot be redirected.

## The wires that work

1. **Outbound webhooks — push, config-only, complete.**
   `agent/outbound_webhooks.py` registers HTTP POST callbacks onto
   the plugin hook manager from config alone (`:156-205`), wired
   at gateway startup (`gateway/run.py:13183-13186`):

   ```yaml
   hooks:
     outbound:
       - url: http://127.0.0.1:<harness-port>/hermes-events
         events: [subagent_start, subagent_stop, post_llm_call,
                  on_session_end]
   ```

   - `subagent_stop` fires for EVERY child, background included —
     from `_finalize_child_results` on the batch thread
     (`delegate_tool.py:3463`, `:3480-3494`) — with
     `parent_session_id` (the run's session), `child_session_id`,
     `child_summary`, `child_status`, `tool_call_history`,
     `duration_ms`.
   - `post_llm_call` fires once per turn from the finalizer
     (`agent/turn_finalizer.py:636-648`) with `session_id`,
     `user_message`, `assistant_response`, and
     `conversation_history` — the whole turn's messages. For the
     wake turn that is: the consolidated block as the user
     message, the parent's follow-up as the response, and every
     tool call and result the follow-up made in between.
   HMAC-signed, bounded queue, daemon worker; `HERMES_SAFE_MODE=1`
   disables it (`:169-172`) — never set that. Valid events:
   `hermes_cli/plugins.py:163-227`.
2. **The run's own `subagent.complete`** for children that finish
   inside the parent turn — free.
3. **`GET /api/sessions/{session_id}/messages`** (route
   `api_server.py:2246`, handler `:4495-4552`) — the durable
   transcript, wake turn included. Pull.
4. **state.db `async_delegations`** — `result_json` per batch,
   keyed by `origin_session_id`. Pull, authoritative.
5. **Live transcripts** — `$HERMES_HOME/cache/delegation/live/
   <deleg_*>/task-<n>.log` + `manifest.json`, pre-created at
   dispatch for tailing (`tools/delegation_live_log.py`). Detail,
   no summaries.

## A design that supports delegation

The container's stream is OURS to hold open — the run's is not.
After `run.completed`, if delegations are outstanding (the
harness counts `subagent.start` against `subagent_stop`, or reads
`async_delegations.delivery_state`), keep the container stream
open and wait for the wake turn's `post_llm_call`; render that
turn from its `conversation_history` — the consolidated block as
a user chunk, the follow-up's tool calls and responses, its text —
then repeat, since a follow-up may delegate again; finish when
nothing is outstanding. Children still get live
`subagent.complete` events in-turn, and MCP tool calls in wake
turns still transit the proxy in real time; only built-in tool
activity in a wake turn arrives at turn end rather than live.
Requirements: `API_SERVER_KEY` set (the wake dies without it), a
`max_concurrent_runs` that admits the self-POST beside the run,
and an explicit `session_id` on `POST /v1/runs` so the wake target
is known up front.

## A finding that outranks delegation

`POST /v1/runs` does NOT resume a session's history. It builds
`conversation_history` only from the request body or
`previous_response_id` (`api_server.py:7623-7665`); unlike
`/v1/chat/completions` (`:5150`) and the session chat stream
(`:4838`), it never calls `get_messages_as_conversation`. The
agent APPENDS to the session row, so the DB transcript stays
correct, but a second run on the same `session_id` starts blind
unless the harness passes the history itself — from
`GET /api/sessions/{id}/messages` or state.db. That shapes the
continuation design regardless of delegation.
