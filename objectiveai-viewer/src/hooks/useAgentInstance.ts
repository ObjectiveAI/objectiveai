/**
 * One agent's live view from the daemon's `/agents/instances/{*aih}`
 * endpoint: its status record (tags, timestamps, counters, active
 * flag) and its conversation, materialized. Each consumer (an agent
 * node in the hierarchy tree) constructs and owns its OWN listener —
 * no singletons, no shared stores. Reconnects with a 1s pause when
 * the connection drops (the daemon disconnects lagging clients; each
 * reconnect replays a fresh snapshot, replacing everything).
 *
 * The conversation is exposed as its LAST block only — the tree shows
 * "the most recent thing", whatever it is; the full conversation view
 * is the popup's job (later).
 */
import { useEffect, useState } from "react";
import {
  AgentsInstancesListener,
  Client,
  type DaemonAgentsInstancesListenerAgentRecord,
  type DaemonAgentsInstancesListenerConversationBlock,
  type ViewerTransport,
} from "@objectiveai/sdk";
import { reportError } from "../lib/errors";

export type AgentRecord = DaemonAgentsInstancesListenerAgentRecord;
export type ConversationBlock =
  DaemonAgentsInstancesListenerConversationBlock;

export interface AgentInstanceView {
  /** The agent's status record — `null` until the first status frame
   * lands (the daemon ships one right after auth). */
  agent: AgentRecord | null;
  /** The whole conversation, blocks in conversation order. */
  blocks: ConversationBlock[];
  /** The LAST conversation block, whatever it is — `null` while the
   * conversation is empty. */
  lastBlock: ConversationBlock | null;
  /** Whether the snapshot replay has completed. */
  live: boolean;
}

export function useAgentInstance(
  transport: ViewerTransport | null,
  agentInstanceHierarchy: string,
): AgentInstanceView {
  const [view, setView] = useState<AgentInstanceView>({
    agent: null,
    blocks: [],
    lastBlock: null,
    live: false,
  });
  useEffect(() => {
    if (transport === null) return;
    let cancelled = false;
    let current: AgentsInstancesListener | null = null;
    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await Client.viewer(
            transport,
          ).agentsInstancesListener(agentInstanceHierarchy);
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          const push = () => {
            const blocks = listener.conversation();
            const lastBlock =
              blocks.length > 0 ? blocks[blocks.length - 1] : null;
            setView({
              agent: listener.agent(),
              blocks,
              lastBlock,
              live: listener.live,
            });
          };
          push();
          // Ride the connection until it closes (subscribe resolves on
          // every change AND on close), pushing the fold each wake.
          while (!listener.closed) {
            await listener.subscribe();
            if (cancelled) return;
            push();
          }
        } catch (error) {
          // Connect refused / handshake failure — surface it, then retry.
          reportError(`agent ${agentInstanceHierarchy}`, error);
        }
        current = null;
        if (cancelled) return;
        await new Promise((resolve) => setTimeout(resolve, 1000));
      }
    })();
    return () => {
      cancelled = true;
      current?.close();
    };
  }, [transport, agentInstanceHierarchy]);
  return view;
}
