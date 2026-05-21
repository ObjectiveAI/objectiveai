import cn from "classnames";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import { MessageInput } from "./components/shared/MessageInput";
import type { AgentCompletionEntry } from "./types";
import { isLastAssistantDone } from "./lib/typeGuards";

export function AgentCompletionView({ entry }: { entry: AgentCompletionEntry }) {
  const errorForChat = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const isStreaming = entry.chunk && !entry.error && !isLastAssistantDone(entry.chunk.messages);
  const responseId = entry.chunk?.id ?? entry.id;

  return (
    <>
      {!entry.chunk && !entry.error && (
        <div className={cn("max-w-content", "mx-auto", "mb-4", "px-4")}>
          <div className={cn("flex", "items-center", "gap-2", "py-3", "text-xs", "text-info-dim")}>
            <div className={cn("flex", "gap-0.5")}>
              <span className={cn("w-1", "h-3", "bg-copper-bright/60", "rounded-sm", "animate-pulse")} />
              <span className={cn("w-1", "h-3", "bg-copper-bright/40", "rounded-sm", "animate-pulse", "[animation-delay:150ms]")} />
              <span className={cn("w-1", "h-3", "bg-copper-bright/20", "rounded-sm", "animate-pulse", "[animation-delay:300ms]")} />
            </div>
            <span className={cn("font-mono")}>Connecting…</span>
          </div>
        </div>
      )}
      <AgentCompletionChat
        requestMessages={entry.request.messages}
        chunk={entry.chunk}
        error={errorForChat}
        id={entry.id}
      />
      {isStreaming && <MessageInput responseId={responseId} />}
    </>
  );
}
