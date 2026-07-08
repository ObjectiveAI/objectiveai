/**
 * Materialized consumer of the cli daemon's `/agents/instances/{*aih}`
 * endpoint over a NATIVE WebSocket — the JS mirror of the Rust SDK's
 * `cli::websocket_agents_instances_listener::WebSocketAgentsInstancesListener`,
 * identical in construction and semantics.
 *
 * One connection carries TWO structurally independent concerns:
 *
 * - **The conversation** — typed part events (one per conversation
 *   row, each naming its block class and carrying its class's boundary
 *   fields + ONE mirrored part + the DB row identity as an opaque
 *   replace-at key), replayed from the DB snapshot first, one `live`
 *   marker, then live as the agent produces them. Events fold into an
 *   ordered list of {@link ConversationBlock}s — the exact mirror of
 *   `agents logs list`'s `ResponseItem` family with content INLINED.
 *   A re-sent identity REPLACES the prior part (later = more
 *   complete), which also converges the snapshot/live seam.
 * - **The agent's status** — `agent` events carry this agent's list
 *   record (lock-driven active flag, bound tags, counters), once at
 *   connect and on every change. Held separately; the conversation
 *   callback never fires for it and vice versa.
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
  CliWebsocketAgentsInstancesListenerAssistantResponsePart,
  CliWebsocketAgentsInstancesListenerClientNotificationPart,
  CliWebsocketAgentsInstancesListenerConversationBlock,
  CliWebsocketAgentsInstancesListenerRequestMessageUserPart,
  CliWebsocketAgentsInstancesListenerToolResponsePart,
  CliWebsocketAgentsInstancesListenerVectorRequestChoice,
  CliWebsocketAgentsInstancesListenerVectorRequestChoicePart,
} from "./websocket_agents_instances_listener";

type AgentRecord = CliWebsocketAgentsInstancesListenerAgentRecord;
type AgentInstanceEvent = CliWebsocketAgentsInstancesListenerAgentInstanceEvent;
type ConversationBlock = CliWebsocketAgentsInstancesListenerConversationBlock;
type AssistantResponsePart = CliWebsocketAgentsInstancesListenerAssistantResponsePart;
type ClientNotificationPart = CliWebsocketAgentsInstancesListenerClientNotificationPart;
type RequestMessageUserPart = CliWebsocketAgentsInstancesListenerRequestMessageUserPart;
type ToolResponsePart = CliWebsocketAgentsInstancesListenerToolResponsePart;
type VectorRequestChoice = CliWebsocketAgentsInstancesListenerVectorRequestChoice;
type VectorRequestChoicePart = CliWebsocketAgentsInstancesListenerVectorRequestChoicePart;

/** The conversation part-event variants (everything except `live` /
 * `agent` / the single-shot `vector_response_vote` and `error`). */
type PartEvent = Extract<
  AgentInstanceEvent,
  {
    type:
      | "request_message_user_part"
      | "request_message_assistant_part"
      | "request_message_tool_part"
      | "vector_request_choice_part"
      | "client_notification_part"
      | "assistant_response_part"
      | "tool_response_part";
  }
>;

/** One in-progress multi-part block: its boundary key (canonical
 * string — equal keys join on adjacency), block-level fields, and its
 * parts keyed by the row identity for replace-in-place. */
type OpenBlock = {
  kind: "open";
  class: PartEvent["type"];
  keyString: string;
  agent_instance_hierarchy: string;
  response_id: string;
  tool_call_id: string;
  sender: string;
  message_queue_id: number;
  queued_at: string;
  key: string | null;
  parts: Map<
    string,
    {
      rowIndex: number;
      rowSubIndex: number | null;
      choiceKey: string;
      part:
        | AssistantResponsePart
        | ClientNotificationPart
        | RequestMessageUserPart
        | ToolResponsePart
        | VectorRequestChoicePart;
    }
  >;
};

/** A complete single-row block (vote — replaced at identity; error —
 * immutable, deduped by value). */
type SingleBlock = {
  kind: "single";
  block: ConversationBlock;
};

type Slot = OpenBlock | SingleBlock;

/** The block boundary as a canonical string — `read_all`'s exact
 * tuple, typed per class (AIH omitted: constant per connection). */
function blockKeyString(event: PartEvent): string {
  switch (event.type) {
    case "request_message_user_part":
    case "request_message_assistant_part":
    case "assistant_response_part":
    case "vector_request_choice_part":
      return `${event.type}|${event.response_id}`;
    case "request_message_tool_part":
    case "tool_response_part":
      return `${event.type}|${event.response_id}|${event.tool_call_id}`;
    case "client_notification_part":
      return `${event.type}|${event.response_id}|${event.sender_agent_instance_hierarchy}|${event.message_queue_id}`;
  }
}

/** A part's replace-at key (identical between snapshot and live). */
function partIdentity(event: PartEvent): string {
  const [rowIndex, rowSubIndex] = eventIndices(event);
  return `${event.type}|${event.response_id}|${rowIndex}|${rowSubIndex ?? ""}`;
}

function eventIndices(event: PartEvent): [number, number | null] {
  if (event.type === "vector_request_choice_part") {
    return [event.choice_index, event.part_index];
  }
  if (event.type === "client_notification_part") {
    return [event.row_index, null];
  }
  return [event.row_index, event.row_sub_index ?? null];
}

function newOpenBlock(event: PartEvent): OpenBlock {
  return {
    kind: "open",
    class: event.type,
    keyString: blockKeyString(event),
    agent_instance_hierarchy: event.agent_instance_hierarchy,
    response_id: event.response_id,
    tool_call_id:
      event.type === "request_message_tool_part" ||
      event.type === "tool_response_part"
        ? event.tool_call_id
        : "",
    sender:
      event.type === "client_notification_part"
        ? event.sender_agent_instance_hierarchy
        : "",
    message_queue_id:
      event.type === "client_notification_part" ? event.message_queue_id : 0,
    queued_at: event.type === "client_notification_part" ? event.queued_at : "",
    key: null,
    parts: new Map(),
  };
}

/** Fold one part event into its block: block-level metadata refreshes
 * (constant per block by construction), the part upserts its slot. */
function applyToBlock(block: OpenBlock, event: PartEvent): void {
  const [rowIndex, rowSubIndex] = eventIndices(event);
  if (event.type === "client_notification_part") {
    block.queued_at = event.queued_at;
    block.key = event.key ?? null;
  }
  block.parts.set(partIdentity(event), {
    rowIndex,
    rowSubIndex,
    choiceKey: event.type === "vector_request_choice_part" ? event.key : "",
    part: event.part,
  });
}

/** Sorted part entries — `(row_index, row_sub_index [null first])`. */
function sortedParts(block: OpenBlock) {
  return [...block.parts.values()].sort((a, b) => {
    if (a.rowIndex !== b.rowIndex) return a.rowIndex - b.rowIndex;
    const aSub = a.rowSubIndex ?? -Infinity;
    const bSub = b.rowSubIndex ?? -Infinity;
    return aSub < bSub ? -1 : aSub > bSub ? 1 : 0;
  });
}

/** Materialize one open block as its `ResponseItem`-mirror shape. */
function toBlock(block: OpenBlock): ConversationBlock | null {
  if (block.parts.size === 0) return null;
  const base = {
    agent_instance_hierarchy: block.agent_instance_hierarchy,
    response_id: block.response_id,
  };
  switch (block.class) {
    case "request_message_user_part":
      return {
        type: "request_message_user",
        ...base,
        parts: sortedParts(block).map(
          (entry) => entry.part as RequestMessageUserPart,
        ),
      };
    case "request_message_assistant_part":
      return {
        type: "request_message_assistant",
        ...base,
        parts: sortedParts(block).map(
          (entry) => entry.part as AssistantResponsePart,
        ),
      };
    case "assistant_response_part":
      return {
        type: "assistant_response",
        ...base,
        parts: sortedParts(block).map(
          (entry) => entry.part as AssistantResponsePart,
        ),
      };
    case "request_message_tool_part":
      return {
        type: "request_message_tool",
        ...base,
        tool_call_id: block.tool_call_id,
        parts: sortedParts(block).map(
          (entry) => entry.part as ToolResponsePart,
        ),
      };
    case "tool_response_part":
      return {
        type: "tool_response",
        ...base,
        tool_call_id: block.tool_call_id,
        parts: sortedParts(block).map(
          (entry) => entry.part as ToolResponsePart,
        ),
      };
    case "client_notification_part":
      return {
        type: "client_notification",
        ...base,
        sender_agent_instance_hierarchy: block.sender,
        queued_at: block.queued_at,
        ...(block.key !== null ? { key: block.key } : {}),
        parts: sortedParts(block).map(
          (entry) => entry.part as ClientNotificationPart,
        ),
      };
    case "vector_request_choice_part": {
      // Ordered by (choice index, part index); group consecutive runs
      // of one choice index.
      const choices: VectorRequestChoice[] = [];
      let current: { index: number; choice: VectorRequestChoice } | null =
        null;
      for (const entry of sortedParts(block)) {
        if (current !== null && current.index === entry.rowIndex) {
          current.choice.key = entry.choiceKey;
          current.choice.parts.push(entry.part as VectorRequestChoicePart);
        } else {
          if (current !== null) choices.push(current.choice);
          current = {
            index: entry.rowIndex,
            choice: {
              key: entry.choiceKey,
              parts: [entry.part as VectorRequestChoicePart],
            },
          };
        }
      }
      if (current !== null) choices.push(current.choice);
      return { type: "vector_request_choices", ...base, choices };
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
 * listener.conversation(); // ConversationBlock[] — the logs-list mirror
 * listener.agent();        // AgentRecord | null — active, tags, counters
 * listener.live;           // snapshot replay complete?
 * await listener.subscribe(); // resolves on the next change (either concern)
 * ```
 */
export class WebSocketAgentsInstancesListener {
  #ws: WebSocket;
  #closed = false;
  #slots: Slot[] = [];
  #partToSlot = new Map<string, number>();
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

  /** The current conversation, blocks in conversation order — the
   * `agents logs list` `ResponseItem` mirror, content inlined. */
  conversation(): ConversationBlock[] {
    const out: ConversationBlock[] = [];
    for (const slot of this.#slots) {
      if (slot.kind === "single") {
        out.push(slot.block);
        continue;
      }
      const materialized = toBlock(slot);
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
      case "live":
        this.#live = true;
        this.#onChange?.(this.conversation());
        break;
      case "agent":
        this.#agent = event.agent;
        this.#onAgentChange?.(event.agent);
        break;
      case "vector_response_vote":
        this.#applySingle(`vote|${event.response_id}`, {
          type: "vector_response_vote",
          agent_instance_hierarchy: event.agent_instance_hierarchy,
          response_id: event.response_id,
          vote: event.vote,
        });
        this.#onChange?.(this.conversation());
        break;
      case "error":
        this.#applyError({
          type: "error",
          agent_instance_hierarchy: event.agent_instance_hierarchy,
          ...(event.response_id != null
            ? { response_id: event.response_id }
            : {}),
          error: event.error,
          delivered_at: event.delivered_at,
        });
        this.#onChange?.(this.conversation());
        break;
      case "request_message_user_part":
      case "request_message_assistant_part":
      case "request_message_tool_part":
      case "vector_request_choice_part":
      case "client_notification_part":
      case "assistant_response_part":
      case "tool_response_part":
        this.#applyPart(event);
        this.#onChange?.(this.conversation());
        break;
      default:
        // Unknown event kind — skipped (forward compat).
        return;
    }
    this.#wake();
  }

  /** Fold one part event: a known identity routes straight back to its
   * slot (replace in place — never re-runs boundary logic); a new
   * identity joins the LAST slot iff it is an open block with an EQUAL
   * boundary key, else opens a new block. Block order = arrival order
   * = conversation order. */
  #applyPart(event: PartEvent): void {
    const identity = partIdentity(event);
    let target = this.#partToSlot.get(identity);
    if (target === undefined) {
      const keyString = blockKeyString(event);
      const last = this.#slots[this.#slots.length - 1];
      if (last !== undefined && last.kind === "open" && last.keyString === keyString) {
        target = this.#slots.length - 1;
      } else {
        this.#slots.push(newOpenBlock(event));
        target = this.#slots.length - 1;
      }
      this.#partToSlot.set(identity, target);
    }
    const slot = this.#slots[target];
    if (slot.kind !== "open") return; // impossible by class
    applyToBlock(slot, event);
  }

  /** Fold one complete single-row block: replace at a known identity,
   * else append. */
  #applySingle(identity: string, block: ConversationBlock): void {
    const target = this.#partToSlot.get(identity);
    if (target !== undefined) {
      this.#slots[target] = { kind: "single", block };
      return;
    }
    this.#slots.push({ kind: "single", block });
    this.#partToSlot.set(identity, this.#slots.length - 1);
  }

  /** Fold one error block. Errors are IMMUTABLE and carry no
   * replace-at identity — the snapshot/live seam (the one way the same
   * error arrives twice) dedupes by VALUE equality. */
  #applyError(block: ConversationBlock): void {
    const encoded = JSON.stringify(block);
    for (const slot of this.#slots) {
      if (slot.kind === "single" && JSON.stringify(slot.block) === encoded) {
        return;
      }
    }
    this.#slots.push({ kind: "single", block });
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
