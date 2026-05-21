import { useCallback, type Dispatch, type SetStateAction } from "react";
import cn from "classnames";
import type {
  AgentCompletionsResponseStreamingAssistantResponseChunk,
  AgentCompletionsResponseToolResponse,
} from "@objectiveai/sdk";
import type { PanelTab, PanelTabCompletionEntry } from "../RightOverlayPanel";
import { UserBubble } from "./UserBubble";
import { AssistantBubble } from "./AssistantBubble";
import { ToolResponseBlock } from "./ToolResponseBlock";
import { parseToolResponse } from "./parseToolResponse";
import { toolResponseKey } from "./dropdownKey";

interface MessageListProps {
  tab: PanelTab;
  setPanelTabs: Dispatch<SetStateAction<PanelTab[]>>;
}

export function MessageList({ tab, setPanelTabs }: MessageListProps) {
  const setOverride = useCallback(
    (key: string, open: boolean) => {
      setPanelTabs((prev) =>
        prev.map((t) =>
          t.id === tab.id
            ? {
                ...t,
                dropdownOverrides: { ...t.dropdownOverrides, [key]: open },
              }
            : t,
        ),
      );
    },
    [tab.id, setPanelTabs],
  );

  // Count user-message-notifications extracted across all tool
  // responses; the visual queue at the bottom is whatever prefix of
  // notifyHistory hasn't been absorbed yet (FIFO).
  let totalExtracted = 0;
  for (const entry of tab.entries) {
    if (entry.kind !== "completion" || !entry.chunk) continue;
    for (const m of entry.chunk.messages) {
      if (m.role !== "tool") continue;
      const tool = m as AgentCompletionsResponseToolResponse;
      totalExtracted += parseToolResponse(tool.content).extractedUserMessages
        .length;
    }
  }
  const visualQueue = tab.notifyHistory.slice(totalExtracted);

  if (tab.entries.length === 0 && visualQueue.length === 0) {
    return (
      <div
        className={cn(
          "flex",
          "flex-1",
          "items-center",
          "justify-center",
          "text-sm",
          "text-neutral-500",
          "dark:text-neutral-400",
          "italic",
        )}
      >
        Say hi to {tab.favorite.name}.
      </div>
    );
  }

  return (
    <div className={cn("flex", "flex-col", "gap-3", "px-3", "py-4")}>
      {tab.entries.map((entry, i) => {
        if (entry.kind === "user") {
          return <UserBubble key={i} msg={entry.message} />;
        }
        return (
          <CompletionBlock
            key={i}
            entry={entry}
            overrides={tab.dropdownOverrides}
            setOverride={setOverride}
          />
        );
      })}

      {visualQueue.map((msg, i) => (
        <div key={`q-${i}`} className={cn("opacity-60", "italic")}>
          <UserBubble msg={msg} />
        </div>
      ))}
    </div>
  );
}

interface CompletionBlockProps {
  entry: PanelTabCompletionEntry;
  overrides: Record<string, boolean>;
  setOverride: (key: string, open: boolean) => void;
}

function CompletionBlock({ entry, overrides, setOverride }: CompletionBlockProps) {
  const chunk = entry.chunk;

  // Pre-index tool-call ids → function name so tool responses can
  // title themselves `<tool> response`.
  const toolNameById = new Map<string, string>();
  if (chunk) {
    for (const m of chunk.messages) {
      if (m.role !== "assistant") continue;
      const asst = m as AgentCompletionsResponseStreamingAssistantResponseChunk;
      for (const call of asst.tool_calls ?? []) {
        if (call.id) {
          toolNameById.set(call.id, call.function?.name ?? "unknown");
        }
      }
    }
  }

  return (
    <div className={cn("flex", "flex-col", "gap-3")}>
      {chunk?.messages.map((m, mi) => {
        if (m.role === "assistant") {
          return (
            <AssistantBubble
              key={mi}
              msg={m as AgentCompletionsResponseStreamingAssistantResponseChunk}
              streaming={entry.streaming}
              entryRequestId={entry.requestId}
              msgIndex={mi}
              overrides={overrides}
              setOverride={setOverride}
            />
          );
        }
        if (m.role === "tool") {
          const tool = m as AgentCompletionsResponseToolResponse;
          const toolName =
            toolNameById.get(tool.tool_call_id) ?? "unknown";
          const { cleanContent, extractedUserMessages } = parseToolResponse(
            tool.content,
          );
          return (
            <div key={mi} className={cn("flex", "flex-col", "gap-2")}>
              <ToolResponseBlock
                toolName={toolName}
                content={cleanContent}
                dropdownKey={toolResponseKey(entry.requestId, mi)}
                overrides={overrides}
                setOverride={setOverride}
              />
              {extractedUserMessages.map((u, ui) => (
                <UserBubble key={`x-${ui}`} msg={u} />
              ))}
            </div>
          );
        }
        return null;
      })}

      {entry.streaming && entry.chunk === null && (
        <div
          className={cn(
            "self-start",
            "text-xs",
            "italic",
            "text-neutral-500",
            "dark:text-neutral-400",
          )}
        >
          …
        </div>
      )}

      {entry.error && (
        <div
          className={cn(
            "self-stretch",
            "rounded",
            "border",
            "border-red-200",
            "dark:border-red-800",
            "bg-red-50",
            "dark:bg-red-950",
            "text-red-700",
            "dark:text-red-300",
            "px-3",
            "py-2",
            "text-xs",
          )}
        >
          Error {entry.error.code}: {JSON.stringify(entry.error.message)}
        </div>
      )}
    </div>
  );
}
