import { useCallback, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  ErrorResponseErrorSchema,
  ViewerEventSchema,
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  type AgentCompletionsMessageMessage,
  type AgentCompletionsMessageRichContent,
  type AgentCompletionsMessageRichContentPart,
  type RemotePathCommitOptional,
  type ViewerEvent,
} from "@objectiveai/sdk";
import { buildAgentCompletionRequest } from "./buildAgentCompletionRequest";
import type { PanelTab, PanelTabCompletionEntry, PanelTabEntry } from "../RightOverlayPanel";

interface UseAgentChat {
  sendMessage: (tabId: string) => void;
}

export function useAgentChat(
  setPanelTabs: Dispatch<SetStateAction<PanelTab[]>>,
): UseAgentChat {
  const fireNotify = useCallback(
    async (responseId: string, content: AgentCompletionsMessageRichContent) => {
      const params = { response_id: responseId, content };
      const origin = `agent-completion-notify-${crypto.randomUUID()}`;
      let unlisten: UnlistenFn | undefined;
      unlisten = await listen<ViewerEvent>(origin, (ev) => {
        const ce = ViewerEventSchema.safeParse(ev.payload);
        if (!ce.success || ce.data.type !== "cli_command") return;
        const line = ce.data.value as { type?: string };
        if (line?.type === "end") unlisten?.();
      });
      invoke("cli_run", {
        args: [
          "api", "agent", "completions", "notify", "post",
          "--body-inline", JSON.stringify(params),
        ],
        origin,
      });
    },
    [],
  );

  const startStream = useCallback(
    async (
      tabId: string,
      requestId: string,
      agent: RemotePathCommitOptional,
      messages: AgentCompletionsMessageMessage[],
      continuation: string | null,
    ) => {
      const origin = `agent-completion-${requestId}`;
      const request = buildAgentCompletionRequest(agent, messages, continuation);

      let unlisten: UnlistenFn | undefined;
      unlisten = await listen<ViewerEvent>(origin, (ev) => {
        const ce = ViewerEventSchema.safeParse(ev.payload);
        if (!ce.success || ce.data.type !== "cli_command") return;
        const line = ce.data.value as { type?: string; value?: unknown };

        if (line.type === "notification") {
          const parsed = AgentCompletionsResponseStreamingAgentCompletionChunkSchema
            .safeParse(line.value);
          if (!parsed.success) return;
          const next = parsed.data;
          const toFlush: AgentCompletionsMessageRichContent[] = [];
          let flushedResponseId: string | null = null;
          setPanelTabs((prev) => prev.map((t) => {
            if (t.id !== tabId || t.inFlightEntryIndex === null) return t;
            const idx = t.inFlightEntryIndex;
            const entry = t.entries[idx];
            if (!entry || entry.kind !== "completion" || entry.requestId !== requestId) {
              return t;
            }
            const wasNull = entry.chunk === null;
            const merged = entry.chunk
              ? agentCompletionsResponseStreamingAgentCompletionChunkMerged(
                  entry.chunk, next,
                )[0]
              : next;
            const entries = t.entries.slice();
            entries[idx] = { ...entry, chunk: merged };

            if (wasNull && merged.id && t.pendingNotifyContent.length > 0) {
              flushedResponseId = merged.id;
              toFlush.push(...t.pendingNotifyContent);
              return { ...t, entries, pendingNotifyContent: [] };
            }
            return { ...t, entries };
          }));
          if (flushedResponseId !== null) {
            const rid = flushedResponseId;
            for (const content of toFlush) {
              queueMicrotask(() => fireNotify(rid, content));
            }
          }
        } else if (line.type === "error") {
          const parsed = ErrorResponseErrorSchema.safeParse(line.value);
          if (!parsed.success) return;
          setPanelTabs((prev) => prev.map((t) => {
            if (t.id !== tabId || t.inFlightEntryIndex === null) return t;
            const idx = t.inFlightEntryIndex;
            const entry = t.entries[idx];
            if (!entry || entry.kind !== "completion" || entry.requestId !== requestId) {
              return t;
            }
            const entries = t.entries.slice();
            entries[idx] = { ...entry, error: parsed.data };
            return { ...t, entries };
          }));
        } else if (line.type === "end") {
          unlisten?.();
          finalizeStream(tabId, requestId);
        }
      });

      invoke("cli_run", {
        args: [
          "api", "agent", "completions", "post",
          "--body-inline", JSON.stringify(request),
        ],
        origin,
      });
    },
    [setPanelTabs, fireNotify],
  );

  const finalizeStream = useCallback(
    (tabId: string, requestId: string) => {
      let scheduleDrain: { agent: RemotePathCommitOptional; continuation: string | null; drainRequestId: string } | null = null;
      setPanelTabs((prev) => prev.map((t) => {
        if (t.id !== tabId || t.inFlightEntryIndex === null) return t;
        const idx = t.inFlightEntryIndex;
        const entry = t.entries[idx];
        if (!entry || entry.kind !== "completion" || entry.requestId !== requestId) {
          return t;
        }
        const finalChunk = entry.chunk;
        const continuation = finalChunk?.continuation ?? t.continuation;
        const messagesQueued = finalChunk?.messages_queued === true;
        const entries = t.entries.slice();
        entries[idx] = { ...entry, streaming: false };

        if (messagesQueued) {
          const drainRequestId = crypto.randomUUID();
          const drainEntry: PanelTabCompletionEntry = {
            kind: "completion",
            chunk: null,
            streaming: true,
            error: null,
            requestId: drainRequestId,
          };
          entries.push(drainEntry);
          scheduleDrain = { agent: t.agent, continuation, drainRequestId };
          return {
            ...t,
            entries,
            continuation,
            inFlightEntryIndex: entries.length - 1,
            pendingNotifyContent: [],
          };
        }

        return {
          ...t,
          entries,
          continuation,
          inFlightEntryIndex: null,
          pendingNotifyContent: [],
        };
      }));
      if (scheduleDrain) {
        const { agent, continuation, drainRequestId } = scheduleDrain;
        queueMicrotask(() => startStream(tabId, drainRequestId, agent, [], continuation));
      }
    },
    [setPanelTabs, startStream],
  );

  const sendMessage = useCallback(
    (tabId: string) => {
      let startFresh:
        | { agent: RemotePathCommitOptional; continuation: string | null; requestId: string; userMsg: AgentCompletionsMessageMessage }
        | null = null;
      let notifyNow: { responseId: string; content: AgentCompletionsMessageRichContent } | null = null;
      setPanelTabs((prev) => prev.map((t) => {
        if (t.id !== tabId) return t;
        const built = buildUserMessage(t.draft, t.attachments);
        if (built === null) return t;
        const cleared = { ...t, draft: "", attachments: [] };
        const userEntry: PanelTabEntry = { kind: "user", message: built };

        if (t.inFlightEntryIndex !== null) {
          // Mid-flight: don't append to entries; push onto
          // notifyHistory (renders as a dimmed bubble at the
          // chat bottom). Once the agent's tool response brings
          // it back as a <system-reminder>, it'll naturally
          // appear in chronological position and the dimmed
          // bubble will disappear.
          const inFlight = t.entries[t.inFlightEntryIndex];
          const content = (built as { content: AgentCompletionsMessageRichContent }).content;
          if (inFlight?.kind === "completion" && inFlight.chunk?.id) {
            notifyNow = { responseId: inFlight.chunk.id, content };
            return { ...cleared, notifyHistory: [...t.notifyHistory, built] };
          }
          return {
            ...cleared,
            notifyHistory: [...t.notifyHistory, built],
            pendingNotifyContent: [...t.pendingNotifyContent, content],
          };
        }

        // Idle: append user entry + completion entry atomically, set inFlight.
        const requestId = crypto.randomUUID();
        const completionEntry: PanelTabEntry = {
          kind: "completion",
          chunk: null,
          streaming: true,
          error: null,
          requestId,
        };
        const entries = [...t.entries, userEntry, completionEntry];
        startFresh = {
          agent: t.agent,
          continuation: t.continuation,
          requestId,
          userMsg: built,
        };
        return {
          ...cleared,
          entries,
          inFlightEntryIndex: entries.length - 1,
          pendingNotifyContent: [],
        };
      }));
      if (startFresh) {
        const { agent, continuation, requestId, userMsg } = startFresh;
        queueMicrotask(() =>
          startStream(tabId, requestId, agent, [userMsg], continuation),
        );
      }
      if (notifyNow) {
        const n = notifyNow as { responseId: string; content: AgentCompletionsMessageRichContent };
        queueMicrotask(() => fireNotify(n.responseId, n.content));
      }
    },
    [setPanelTabs, startStream, fireNotify],
  );

  return { sendMessage };
}

function buildUserMessage(
  draft: string,
  attachments: AgentCompletionsMessageRichContentPart[],
): AgentCompletionsMessageMessage | null {
  const parts: AgentCompletionsMessageRichContentPart[] = [];
  if (draft.trim().length > 0) parts.push({ type: "text", text: draft });
  parts.push(...attachments);
  if (parts.length === 0) return null;
  return { role: "user", content: parts } as AgentCompletionsMessageMessage;
}
