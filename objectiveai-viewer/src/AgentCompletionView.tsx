import type { AgentCompletionsMessageMessage } from "objectiveai";
import type { AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseStreamingAssistantResponseChunk } from "objectiveai";
import { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
import { MessageInput } from "./components/shared/MessageInput";

interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: { id: string; messages?: AgentCompletionsMessageMessage[] };
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

export function AgentCompletionView({ entry }: { entry: AgentCompletionEntry }) {
  const errorForChat = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const isStreaming = entry.chunk && !entry.error && !entry.chunk.messages.some(
    (m) => m.role === "assistant" && (m as AgentCompletionsResponseStreamingAssistantResponseChunk).finish_reason
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

export { AgentCompletionChat } from "./components/shared/AgentCompletionChat";
export type { AgentCompletionChatProps } from "./components/shared/AgentCompletionChat";
