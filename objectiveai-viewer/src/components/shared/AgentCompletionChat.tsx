import { useState, useCallback, useEffect } from "react";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  AgentCompletionsResponseToolResponse,
  AgentCompletionsMessageMessage,
} from "objectiveai";
import * as Tooltip from "@radix-ui/react-tooltip";
import { SystemBanner, UserBubble, AssistantBubble, ToolResultBubble } from "./MessageBubble";
import { UsageFooter } from "./UsageFooter";
import { isAssistantMessage, isLastAssistantDone, hasContent } from "../../lib/typeGuards";

export interface AgentCompletionChatProps {
  label?: string;
  requestMessages?: AgentCompletionsMessageMessage[];
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: { code: number; message: unknown } | null;
  id?: string;
}

export function AgentCompletionChat({
  label,
  requestMessages,
  chunk,
  error,
  id,
}: AgentCompletionChatProps) {
  let model = "";
  let upstream = "";
  if (chunk) {
    for (const msg of chunk.messages) {
      if (isAssistantMessage(msg)) {
        model = msg.model ?? "";
        break;
      }
    }
    upstream = chunk.upstream ?? "";
  }

  const hasFinish = chunk ? isLastAssistantDone(chunk.messages) : false;
  const status = error ? "error" : hasFinish ? "complete" : "streaming";

  const created = chunk?.created;
  const timeStr = created ? new Date(created * 1000).toLocaleTimeString() : "";
  const displayId = id ?? chunk?.id ?? "";

  const statusColor = {
    streaming: "bg-copper-hot",
    complete: "bg-success",
    error: "bg-error",
  }[status];

  return (
    <div className="text-left max-w-content mx-auto mb-6 border border-node-border rounded-md overflow-hidden text-sm bg-ground-raised">
      {label && (
        <div className="px-4 py-2 bg-ground-surface border-b border-node-border font-mono text-xs font-semibold text-copper-mid break-words">
          {label}
        </div>
      )}

      <div className="flex items-center gap-2.5 px-4 py-2.5 bg-ground-surface border-b border-node-border text-xs text-info-dim">
        <div className={`w-2 h-2 rounded-full shrink-0 ${statusColor} ${status === "streaming" ? "animate-pulse" : ""}`} />
        {model && <span className="font-semibold text-info-bright">{model}</span>}
        {upstream && <span className="bg-ground px-1.5 py-px rounded-sm text-[11px] lowercase text-info-dim">{upstream}</span>}
        {timeStr && <span>{timeStr}</span>}
        {displayId && <CopyableId id={displayId} />}
      </div>

      <div className="p-4 flex flex-col gap-3">
        {requestMessages?.map((msg, i) => {
          if (msg.role === "developer" || msg.role === "system") {
            return (
              <SystemBanner
                key={`req-${i}`}
                role={msg.role}
                content={hasContent(msg) ? msg.content : ""}
              />
            );
          }
          if (msg.role === "user") {
            return <UserBubble key={`req-${i}`} content={hasContent(msg) ? msg.content : ""} />;
          }
          if (isAssistantMessage(msg)) {
            return <AssistantBubble key={`req-${i}`} msg={msg} />;
          }
          return null;
        })}

        {chunk?.messages.map((msg, i) => {
          if (isAssistantMessage(msg)) {
            return <AssistantBubble key={`resp-${i}`} msg={msg} />;
          }
          if (msg.role === "tool") {
            return <ToolResultBubble key={`resp-${i}`} msg={msg as AgentCompletionsResponseToolResponse} />;
          }
          return null;
        })}

        {!chunk && !error && (
          <div className="text-info-dim italic">Waiting for response...</div>
        )}
      </div>

      {error && (
        <div role="alert" className="bg-error/10 border-t border-error/30 px-4 py-2 text-error text-xs">
          Error {error.code}: {JSON.stringify(error.message)}
        </div>
      )}

      {chunk?.usage && <UsageFooter usage={chunk.usage} />}
    </div>
  );
}

function CopyableId({ id }: { id: string }) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const t = setTimeout(() => setCopied(false), 1500);
    return () => clearTimeout(t);
  }, [copied]);

  const copy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(id);
      setCopied(true);
    } catch { /* clipboard denied */ }
  }, [id]);

  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          className="font-mono text-[11px] opacity-60 ml-auto cursor-pointer hover:opacity-100 transition-opacity"
          onClick={copy}
        >
          {id.slice(0, 12)}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content className="bg-ground-surface border border-node-border rounded-sm px-2 py-1 font-mono text-[11px] text-info-mid" sideOffset={4}>
          {copied ? "Copied!" : id}
          <Tooltip.Arrow className="fill-ground-surface" />
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}
