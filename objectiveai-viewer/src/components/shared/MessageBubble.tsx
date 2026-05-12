import type {
  AgentCompletionsResponseStreamingAssistantResponseChunk,
  AgentCompletionsResponseToolResponse,
} from "objectiveai";
import * as Collapsible from "@radix-ui/react-collapsible";
import { RichContent } from "./RichContent";
import { ToolCallCard } from "./ToolCallCard";

export function SystemBanner({ role, content }: { role: string; content: unknown }) {
  const text =
    typeof content === "string"
      ? content
      : Array.isArray(content)
        ? content.map((p: { text?: string }) => p.text ?? "").join("")
        : "";
  return (
    <Collapsible.Root defaultOpen={false}>
      <div className="bg-ground-surface border border-node-border rounded-md px-3 py-2 text-xs text-info-dim">
        <Collapsible.Trigger className="w-full text-left cursor-pointer">
          <div className="font-semibold uppercase text-[10px] tracking-wide text-info-dim flex items-center gap-1.5">
            <span className="text-copper-dim text-[8px]">▶</span>
            {role}
          </div>
        </Collapsible.Trigger>
        <Collapsible.Content>
          <div className="font-mono text-xs whitespace-pre-wrap break-words max-h-[200px] overflow-y-auto text-info-mid mt-1">
            {text}
          </div>
        </Collapsible.Content>
      </div>
    </Collapsible.Root>
  );
}

export function UserBubble({ content }: { content: unknown }) {
  return (
    <div className="self-end max-w-3/4 bg-ground-surface border border-node-border rounded-xl rounded-br-sm px-3.5 py-2.5">
      <div className="text-[10px] font-semibold uppercase tracking-wide text-info-dim mb-1">User</div>
      <RichContent content={content} />
    </div>
  );
}

export type AssistantResponseChunkLike = AgentCompletionsResponseStreamingAssistantResponseChunk;

export function AssistantBubble({ msg }: { msg: AssistantResponseChunkLike }) {
  return (
    <div className="self-start max-w-[85%] bg-ground-raised border border-node-border rounded-xl rounded-bl-sm px-3.5 py-2.5">
      <div className="text-[10px] text-info-dim mb-1.5 flex gap-2 items-center">
        <span className="font-semibold uppercase tracking-wide">Assistant</span>
        {msg.model && <span className="text-info-mid">{msg.model}</span>}
        {msg.agent && <span className="font-mono text-[10px] text-copper-dim">{msg.agent}</span>}
        {msg.finish_reason && <FinishBadge reason={msg.finish_reason} />}
      </div>

      {msg.reasoning && (
        <Collapsible.Root defaultOpen={false}>
          <div className="bg-ground-surface border border-node-border rounded-sm px-2.5 py-2 mb-2 text-xs text-copper-dim italic">
            <Collapsible.Trigger className="w-full text-left cursor-pointer">
              <div className="font-semibold not-italic text-[10px] uppercase tracking-wide text-copper-dim flex items-center gap-1.5">
                <span className="text-[8px]">▶</span>
                Thinking
              </div>
            </Collapsible.Trigger>
            <Collapsible.Content>
              <div className="whitespace-pre-wrap break-words mt-1 max-h-[300px] overflow-y-auto">{msg.reasoning}</div>
            </Collapsible.Content>
          </div>
        </Collapsible.Root>
      )}

      {msg.refusal && (
        <div className="text-error bg-error/10 rounded-sm px-2.5 py-2 text-[13px]">{msg.refusal}</div>
      )}

      {msg.content && (
        <div>
          <RichContent content={msg.content} />
        </div>
      )}

      {msg.tool_calls && msg.tool_calls.length > 0 && (
        <div className="flex flex-col gap-1.5 mt-2">
          {msg.tool_calls.map((tc, i) => (
            <ToolCallCard key={i} call={tc} />
          ))}
        </div>
      )}
    </div>
  );
}

function FinishBadge({ reason }: { reason: string }) {
  const colors: Record<string, string> = {
    stop: "bg-green-900/40 text-green-400",
    length: "bg-amber-900/40 text-amber-400",
    tool_calls: "bg-blue-900/40 text-blue-400",
    content_filter: "bg-red-900/40 text-red-400",
    error: "bg-red-900/40 text-red-400",
  };
  return (
    <span className={`inline-block text-[10px] px-1.5 py-px rounded-sm font-semibold lowercase ${colors[reason] ?? "bg-ground-surface text-info-dim"}`}>
      {reason}
    </span>
  );
}

export function ToolResultBubble({ msg }: { msg: AgentCompletionsResponseToolResponse }) {
  const raw = (msg as unknown as { content: unknown }).content;
  const content =
    typeof raw === "string"
      ? raw
      : Array.isArray(raw)
        ? raw.map((p: unknown) => (typeof p === "object" && p && "text" in p ? (p as { text: string }).text : "")).join("")
        : JSON.stringify(raw);
  return (
    <div className="self-start max-w-[80%] ml-6 bg-ground-surface border border-node-border rounded-lg px-3 py-2 text-xs">
      <div className="font-mono text-[10px] text-copper-mid mb-1">{msg.tool_call_id}</div>
      <div className="font-mono text-[11px] whitespace-pre-wrap break-words max-h-[200px] overflow-y-auto text-info-mid">
        {content}
      </div>
    </div>
  );
}
