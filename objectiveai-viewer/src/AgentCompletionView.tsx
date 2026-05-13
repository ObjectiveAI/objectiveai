import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import { MessageInput } from "./components/shared/MessageInput";
import type { AgentCompletionEntry } from "./types";
import { isAssistantMessage } from "./lib/typeGuards";

export function AgentCompletionView({ entry }: { entry: AgentCompletionEntry }) {
  const errorForChat = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const isStreaming = entry.chunk && !entry.error && !entry.chunk.messages.some(
    (m) => isAssistantMessage(m) && m.finish_reason
  );
  const responseId = entry.chunk?.id ?? entry.id;

  return (
    <>
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

