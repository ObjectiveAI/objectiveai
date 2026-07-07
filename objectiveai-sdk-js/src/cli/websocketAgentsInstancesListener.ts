/**
 * Materialized consumer of the cli daemon's `/agents/instances/{*aih}`
 * endpoint over a NATIVE WebSocket — the JS mirror of the Rust SDK's
 * `cli::websocket_agents_instances_listener::WebSocketAgentsInstancesListener`,
 * identical in construction and semantics.
 *
 * One connection carries TWO structurally independent concerns:
 *
 * - **The conversation** — the agent's full history replayed from the
 *   DB as keyed FULL-VALUE `row` events (never deltas), one `live`
 *   marker, then rows streaming as they occur (teed straight from the
 *   CLI's log writer, not gated on the DB insert). Rows are folded
 *   into an ordered list of blocks mirroring `agents logs list`'s
 *   `ResponseItem` classes with content INLINED. A re-sent row
 *   identity `(table, response_id, row_index, row_sub_index)` REPLACES
 *   the prior value — per-connection ordering means later = more
 *   complete, and the snapshot/live seam converges by the same rule.
 * - **The agent's status** — `agent` events carry this agent's list
 *   record (lock-driven active flag, bound tags, counters; the same
 *   shape `/agents/instances/list` tracks), once at connect and on
 *   every change. Held separately; the conversation callback never
 *   fires for it and vice versa.
 *
 * Auth is the first-message preamble shared by every daemon WebSocket
 * route. One listener = one connection: the daemon DISCONNECTS lagging
 * clients rather than dropping frames, so a closed socket means
 * "reconnect for a fresh snapshot"; the view freezes at its last
 * state. Unparseable events are SKIPPED — the forward-compat contract
 * for future event variants. No runtime validation, like
 * {@link WebSocketListener}.
 */

import type {
  CliWebsocketAgentsInstancesListenerAgentInstanceEvent,
  CliWebsocketAgentsInstancesListenerAgentRecord,
  CliWebsocketAgentsInstancesListenerConversationBlock,
  CliWebsocketAgentsInstancesListenerConversationChoice,
  CliWebsocketAgentsInstancesListenerConversationRow,
} from "./websocket_agents_instances_listener";

type AgentRecord = CliWebsocketAgentsInstancesListenerAgentRecord;
type AgentInstanceEvent = CliWebsocketAgentsInstancesListenerAgentInstanceEvent;
type ConversationBlock = CliWebsocketAgentsInstancesListenerConversationBlock;
type ConversationChoice = CliWebsocketAgentsInstancesListenerConversationChoice;
type ConversationRow = CliWebsocketAgentsInstancesListenerConversationRow;

/** Coarse block-class for a row's `table` — mirrors the CLI reader's
 * classifier (head kinds map to the class they carry metadata for). */
type BlockClass =
  | "client_notification"
  | "assistant_response"
  | "tool_response"
  | "request_message_user"
  | "request_message_assistant"
  | "request_message_tool"
  | "vector_request_choices"
  | "vector_response_vote";

/** Prefix-ordered classifier: more specific prefixes first (the
 * request_message_* families are mutually exclusive). */
function blockClass(table: string): BlockClass {
  if (table.startsWith("message_queue")) return "client_notification";
  if (table.startsWith("assistant_response")) return "assistant_response";
  if (table.startsWith("tool_response")) return "tool_response";
  if (table.startsWith("request_message_user")) return "request_message_user";
  if (table.startsWith("request_message_assistant")) {
    return "request_message_assistant";
  }
  if (table.startsWith("request_message_tool")) return "request_message_tool";
  if (table.startsWith("request_vector_choice")) return "vector_request_choices";
  return "vector_response_vote"; // response_vector_vote
}

/** The Rust `RowTableKind` declaration order — the deterministic
 * tie-break for parts sharing `(row_index, row_sub_index)` (e.g. a
 * refusal and a reasoning on the same message index). */
const TABLE_ORDER: readonly string[] = [
  "message_queue_text",
  "message_queue_image",
  "message_queue_audio",
  "message_queue_video",
  "message_queue_file",
  "assistant_response_refusal",
  "assistant_response_reasoning",
  "assistant_response_tool_calls",
  "assistant_response_content_text",
  "assistant_response_content_image",
  "assistant_response_content_audio",
  "assistant_response_content_video",
  "assistant_response_content_file",
  "tool_response_content_text",
  "tool_response_content_image",
  "tool_response_content_audio",
  "tool_response_content_video",
  "tool_response_content_file",
  "request_message_user_content_text",
  "request_message_user_content_image",
  "request_message_user_content_audio",
  "request_message_user_content_video",
  "request_message_user_content_file",
  "request_message_assistant_refusal",
  "request_message_assistant_reasoning",
  "request_message_assistant_tool_calls",
  "request_message_assistant_content_text",
  "request_message_assistant_content_image",
  "request_message_assistant_content_audio",
  "request_message_assistant_content_video",
  "request_message_assistant_content_file",
  "request_message_tool_content_text",
  "request_message_tool_content_image",
  "request_message_tool_content_audio",
  "request_message_tool_content_video",
  "request_message_tool_content_file",
  "request_vector_choice_content_text",
  "request_vector_choice_content_image",
  "request_vector_choice_content_audio",
  "request_vector_choice_content_video",
  "request_vector_choice_content_file",
  "response_vector_vote",
  "tool_response",
  "request_message_tool",
  "request_vector_choice",
];
const TABLE_INDEX = new Map(TABLE_ORDER.map((name, index) => [name, index]));

/** A row's replace-at key — identical between the snapshot replay and
 * the live tee (AIH is constant per connection and omitted). */
function rowIdentity(row: ConversationRow): string {
  return `${row.table}|${row.response_id}|${row.row_index}|${
    row.row_sub_index ?? ""
  }`;
}

/** One in-progress block. Parts are keyed by identity and sorted by
 * `(row_index, row_sub_index, table order)` at materialize time —
 * arrival-independent, so a late replacement lands in place. */
type BlockState = {
  class: BlockClass;
  agent_instance_hierarchy: string;
  response_id: string;
  tool_call_id: string | null;
  sender: string | null;
  message_queue_id: number | null;
  queued_at: string | null;
  key: string | null;
  vote: number[] | null;
  choiceKeys: Map<number, string>;
  parts: Map<string, ConversationRow>;
};

function newBlock(cls: BlockClass, row: ConversationRow): BlockState {
  return {
    class: cls,
    agent_instance_hierarchy: row.agent_instance_hierarchy,
    response_id: row.response_id,
    tool_call_id: null,
    sender: null,
    message_queue_id: null,
    queued_at: null,
    key: null,
    vote: null,
    choiceKeys: new Map(),
    parts: new Map(),
  };
}

/** Does `row` (a NEW identity of `cls`) belong to this block? The CLI
 * reader's boundary tuple, evaluated against the LAST block: `(class,
 * aih, response_id)` + sender/queue-id for notifications +
 * `tool_call_id` for the tool classes. A live tool CONTENT row carries
 * no `tool_call_id` (only its head does) — adjacency stands in, which
 * is exact because the writer emits head then contents consecutively. */
function accepts(
  block: BlockState,
  cls: BlockClass,
  row: ConversationRow,
): boolean {
  if (
    block.class !== cls ||
    block.agent_instance_hierarchy !== row.agent_instance_hierarchy ||
    block.response_id !== row.response_id
  ) {
    return false;
  }
  if (cls === "client_notification") {
    return (
      block.sender === (row.sender_agent_instance_hierarchy ?? null) &&
      block.message_queue_id === (row.message_queue_id ?? null)
    );
  }
  if (cls === "tool_response" || cls === "request_message_tool") {
    const rowId = row.tool_call_id ?? null;
    if (rowId !== null && block.tool_call_id !== null) {
      return rowId === block.tool_call_id;
    }
    return true; // no id on one side: adjacency decides
  }
  return true;
}

/** Fold one row in: metadata always merges; HEAD/vote rows carry no
 * part; everything else upserts its part slot. */
function applyToBlock(block: BlockState, row: ConversationRow): void {
  if (row.tool_call_id != null) block.tool_call_id = row.tool_call_id;
  if (row.choice_key != null) block.choiceKeys.set(row.row_index, row.choice_key);
  if (row.sender_agent_instance_hierarchy != null) {
    block.sender = row.sender_agent_instance_hierarchy;
  }
  if (row.queued_at != null) block.queued_at = row.queued_at;
  if (row.message_queue_key != null) block.key = row.message_queue_key;
  if (row.message_queue_id != null) block.message_queue_id = row.message_queue_id;
  if (row.content.type === "head") return;
  if (row.content.type === "vote") {
    block.vote = row.content.vote;
    return;
  }
  block.parts.set(rowIdentity(row), row);
}

/** Sorted parts — `(row_index, row_sub_index [null first], table)`. */
function sortedParts(block: BlockState): ConversationRow[] {
  return [...block.parts.values()].sort((a, b) => {
    if (a.row_index !== b.row_index) return a.row_index - b.row_index;
    const aSub = a.row_sub_index ?? -Infinity;
    const bSub = b.row_sub_index ?? -Infinity;
    if (aSub !== bSub) return aSub < bSub ? -1 : 1;
    return (TABLE_INDEX.get(a.table) ?? 0) - (TABLE_INDEX.get(b.table) ?? 0);
  });
}

/** Materialize — `null` while the block has nothing presentable yet
 * (e.g. a lone head row whose contents are in flight). */
function toBlock(block: BlockState): ConversationBlock | null {
  const base = {
    agent_instance_hierarchy: block.agent_instance_hierarchy,
    response_id: block.response_id,
  };
  switch (block.class) {
    case "request_message_user":
    case "request_message_assistant":
    case "assistant_response": {
      if (block.parts.size === 0) return null;
      return { type: block.class, ...base, parts: sortedParts(block) };
    }
    case "request_message_tool":
    case "tool_response": {
      if (block.parts.size === 0) return null;
      return {
        type: block.class,
        ...base,
        tool_call_id: block.tool_call_id ?? "",
        parts: sortedParts(block),
      };
    }
    case "vector_request_choices": {
      if (block.parts.size === 0) return null;
      // Parts iterate ordered by (choice index, part index); group
      // consecutive runs of one choice index.
      const choices: ConversationChoice[] = [];
      let current: { index: number; choice: ConversationChoice } | null = null;
      for (const row of sortedParts(block)) {
        if (current !== null && current.index === row.row_index) {
          current.choice.parts.push(row);
        } else {
          if (current !== null) choices.push(current.choice);
          current = {
            index: row.row_index,
            choice: {
              key: block.choiceKeys.get(row.row_index) ?? "",
              parts: [row],
            },
          };
        }
      }
      if (current !== null) choices.push(current.choice);
      return { type: "vector_request_choices", ...base, choices };
    }
    case "vector_response_vote": {
      if (block.vote === null) return null;
      return { type: "vector_response_vote", ...base, vote: block.vote };
    }
    case "client_notification": {
      if (block.parts.size === 0) return null;
      return {
        type: "client_notification",
        ...base,
        ...(block.sender !== null
          ? { sender_agent_instance_hierarchy: block.sender }
          : {}),
        ...(block.queued_at !== null ? { queued_at: block.queued_at } : {}),
        ...(block.key !== null ? { key: block.key } : {}),
        parts: sortedParts(block),
      };
    }
  }
}

export interface WebSocketAgentsInstancesListenerOptions {
  /** The pre-derived `sha256=<hex(SHA256(DAEMON_SECRET))>`, sent
   * verbatim in the auth preamble. Without it the daemon must be
   * running without a secret. */
  signature?: string | null;
  /** Invoked with the full current conversation (blocks in
   * conversation order) after every applied CONVERSATION event —
   * never for agent-status events. Runs synchronously on frame
   * receipt; for state on demand use
   * {@link WebSocketAgentsInstancesListener.conversation}. */
  onChange?: (blocks: ConversationBlock[]) => void;
  /** Invoked with the agent's refreshed list record after every
   * applied AGENT-STATUS event (activation / deactivation / tag
   * change) — never for conversation events. */
  onAgentChange?: (agent: AgentRecord) => void;
}

/**
 * The materialized `/agents/instances/{*aih}` view — construct via
 * {@link WebSocketAgentsInstancesListener.connect}.
 *
 * ```ts
 * const listener = await WebSocketAgentsInstancesListener.connect(
 *   `ws://127.0.0.1:49152/agents/instances/${aih}`,
 *   { signature, onChange: render, onAgentChange: renderStatus },
 * );
 * listener.conversation(); // ConversationBlock[]
 * listener.agent();        // AgentRecord | null — active, tags, counters
 * listener.live;           // snapshot replay complete?
 * await listener.subscribe(); // resolves on the next change (either concern)
 * ```
 */
export class WebSocketAgentsInstancesListener {
  #ws: WebSocket;
  #closed = false;
  #blocks: BlockState[] = [];
  #rowToBlock = new Map<string, number>();
  #live = false;
  #agent: AgentRecord | null = null;
  #onChange?: (blocks: ConversationBlock[]) => void;
  #onAgentChange?: (agent: AgentRecord) => void;
  #waiters = new Set<() => void>();

  private constructor(
    ws: WebSocket,
    options?: WebSocketAgentsInstancesListenerOptions,
  ) {
    this.#ws = ws;
    this.#onChange = options?.onChange;
    this.#onAgentChange = options?.onAgentChange;
    ws.onmessage = (event: MessageEvent) => {
      if (typeof event.data !== "string") return;
      let frame: unknown;
      try {
        frame = JSON.parse(event.data);
      } catch {
        return;
      }
      this.#onFrame(frame);
    };
    ws.onclose = () => this.#onClose();
  }

  /**
   * Open the connection, send the auth preamble, and resolve once the
   * socket is established (rejects when the daemon is unreachable or
   * closes during the handshake). `url` is the daemon's base address +
   * `/agents/instances/` + the agent's hierarchy (raw slashes are
   * fine). The returned listener immediately begins folding the
   * agent-status frame and the snapshot replay.
   */
  static connect(
    url: string,
    options?: WebSocketAgentsInstancesListenerOptions,
  ): Promise<WebSocketAgentsInstancesListener> {
    return new Promise((resolve, reject) => {
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        reject(
          new Error(`connect daemon agent-instance websocket: ${String(e)}`),
        );
        return;
      }
      ws.onopen = () => {
        // The auth preamble — always the connection's first text
        // frame, `{"signature": null}` against a secretless daemon.
        ws.send(JSON.stringify({ signature: options?.signature ?? null }));
        resolve(new WebSocketAgentsInstancesListener(ws, options));
      };
      ws.onclose = (event: CloseEvent) => {
        reject(
          new Error(
            `connect daemon agent-instance websocket: connection closed` +
              `${event.code ? ` (code ${event.code})` : ""}`,
          ),
        );
      };
    });
  }

  /** Whether the connection has closed (the view is frozen; the daemon
   * disconnects lagging clients — reconnect for a fresh snapshot). */
  get closed(): boolean {
    return this.#closed;
  }

  /** Whether the snapshot replay has completed — every conversation
   * event after the `live` marker is the conversation as it occurs. */
  get live(): boolean {
    return this.#live;
  }

  /** Drop the connection: the view freezes and any pending
   * {@link subscribe} resolves. */
  close(): void {
    if (this.#closed) return;
    this.#ws.close();
    // The browser fires onclose asynchronously; settle deterministically.
    this.#onClose();
  }

  /** The current conversation, blocks in conversation order. */
  conversation(): ConversationBlock[] {
    const out: ConversationBlock[] = [];
    for (const block of this.#blocks) {
      const materialized = toBlock(block);
      if (materialized !== null) out.push(materialized);
    }
    return out;
  }

  /** The agent's current list record (active flag, bound tags,
   * counters) — the same shape `/agents/instances/list` tracks, scoped
   * to this agent. `null` until the connection's first agent-status
   * event lands (the daemon ships one right after auth). Structurally
   * independent of {@link conversation}. */
  agent(): AgentRecord | null {
    return this.#agent;
  }

  /** Resolves on the next applied event — conversation OR agent
   * status. A fresh call waits for the FIRST event after it is made;
   * loop with the getters, or use the callbacks for guaranteed push.
   * Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    if (typeof (frame as { type?: unknown }).type !== "string") return;
    const event = frame as AgentInstanceEvent;
    switch (event.type) {
      case "row":
        this.#applyRow(event.row);
        this.#onChange?.(this.conversation());
        break;
      case "live":
        this.#live = true;
        this.#onChange?.(this.conversation());
        break;
      case "agent":
        this.#agent = event.agent;
        this.#onAgentChange?.(event.agent);
        break;
      default:
        // Unknown event kind — skipped (forward compat).
        return;
    }
    this.#wake();
  }

  /** Fold one row: a known identity routes straight back to its block
   * (replace in place — never re-runs boundary logic); a new identity
   * boundary-tests against the LAST block only, joining it or opening
   * a new one. Block order = arrival order = conversation order. */
  #applyRow(row: ConversationRow): void {
    const identity = rowIdentity(row);
    let target = this.#rowToBlock.get(identity);
    if (target === undefined) {
      const cls = blockClass(row.table);
      const last = this.#blocks[this.#blocks.length - 1];
      if (last !== undefined && accepts(last, cls, row)) {
        target = this.#blocks.length - 1;
      } else {
        this.#blocks.push(newBlock(cls, row));
        target = this.#blocks.length - 1;
      }
      this.#rowToBlock.set(identity, target);
    }
    applyToBlock(this.#blocks[target], row);
  }

  #wake(): void {
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }

  #onClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    this.#wake();
  }
}
