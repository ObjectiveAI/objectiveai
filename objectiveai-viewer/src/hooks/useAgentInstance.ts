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
  WebSocketAgentsInstancesListener,
  type CliWebsocketAgentsInstancesListenerAgentRecord,
  type CliWebsocketAgentsInstancesListenerConversationBlock,
} from "@objectiveai/sdk";
import type { DaemonConnection } from "../lib/daemon";
import { reportError } from "../lib/errors";

export type AgentRecord = CliWebsocketAgentsInstancesListenerAgentRecord;
export type ConversationBlock =
  CliWebsocketAgentsInstancesListenerConversationBlock;

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
  connection: DaemonConnection | null,
  agentInstanceHierarchy: string,
): AgentInstanceView {
  const [view, setView] = useState<AgentInstanceView>({
    agent: null,
    blocks: [],
    lastBlock: null,
    live: false,
  });
  useEffect(() => {
    if (connection === null) return;
    let cancelled = false;
    let current: WebSocketAgentsInstancesListener | null = null;
    void (async () => {
      for (;;) {
        if (cancelled) return;
        try {
          const listener = await WebSocketAgentsInstancesListener.connect(
            `${connection.address}/agents/instances/${agentInstanceHierarchy}`,
            {
              signature: connection.signature,
              onChange: (blocks) => {
                if (cancelled) return;
                const lastBlock =
                  blocks.length > 0 ? blocks[blocks.length - 1] : null;
                setView((previous) => ({
                  ...previous,
                  blocks,
                  lastBlock,
                  live: current?.live ?? previous.live,
                }));
              },
              onAgentChange: (agent) => {
                if (!cancelled) {
                  setView((previous) => ({ ...previous, agent }));
                }
              },
            },
          );
          if (cancelled) {
            listener.close();
            return;
          }
          current = listener;
          // Ride the connection until it closes (subscribe resolves on
          // every change AND on close).
          while (!listener.closed) {
            await listener.subscribe();
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
  }, [connection, agentInstanceHierarchy]);
  return view;
}
