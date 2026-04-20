import "./AgentCompletionView.css";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  AgentCompletionsResponseStreamingAssistantResponseChunk,
  AgentCompletionsResponseToolResponse,
  AgentCompletionsMessageMessage,
  AgentCompletionsMessageRichContentPart,
  AgentCompletionsResponseUsage,
} from "objectiveai";

interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: { id: string; messages?: AgentCompletionsMessageMessage[] };
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

// ---------------------------------------------------------------------------
// Rich content rendering
// ---------------------------------------------------------------------------

function RichContent({ content }: { content: unknown }) {
  if (typeof content === "string") {
    return <span className="ac-bubble-text">{content}</span>;
  }
  if (Array.isArray(content)) {
    return (
      <>
        {content.map((part: AgentCompletionsMessageRichContentPart, i: number) => (
          <RichContentPart key={i} part={part} />
        ))}
      </>
    );
  }
  return null;
}

function RichContentPart({ part }: { part: AgentCompletionsMessageRichContentPart }) {
  if ("text" in part && typeof part.text === "string") {
    return <span className="ac-bubble-text">{part.text}</span>;
  }
  if ("image_url" in part) {
    const url = (part as { image_url: { url: string } }).image_url.url;
    return <img className="ac-image-thumb" src={url} alt="image" />;
  }
  if ("input_audio" in part) {
    return <span className="ac-attachment">Audio attachment</span>;
  }
  if ("video_url" in part) {
    return <span className="ac-attachment">Video attachment</span>;
  }
  if ("file" in part) {
    const file = (part as { file: { filename?: string } }).file;
    return <span className="ac-attachment">{file.filename ?? "File"}</span>;
  }
  return null;
}

// ---------------------------------------------------------------------------
// Message bubbles
// ---------------------------------------------------------------------------

function SystemBanner({ role, content }: { role: string; content: unknown }) {
  const text =
    typeof content === "string"
      ? content
      : Array.isArray(content)
        ? content.map((p: { text?: string }) => p.text ?? "").join("")
        : "";
  return (
    <div className="ac-system-banner">
      <div className="ac-system-label">{role}</div>
      <div className="ac-system-content">{text}</div>
    </div>
  );
}

function UserBubble({ content }: { content: unknown }) {
  return (
    <div className="ac-user-bubble">
      <div className="ac-bubble-role">User</div>
      <RichContent content={content} />
    </div>
  );
}

function AssistantBubble({ msg }: { msg: AssistantResponseChunkLike }) {
  return (
    <div className="ac-assistant-bubble">
      <div className="ac-assistant-meta">
        <span className="ac-bubble-role">Assistant</span>
        {msg.model && <span>{msg.model}</span>}
        {msg.agent && <span className="ac-assistant-agent">{msg.agent}</span>}
        {msg.finish_reason && (
          <span className={`ac-finish-badge ac-finish-${msg.finish_reason}`}>
            {msg.finish_reason}
          </span>
        )}
      </div>

      {msg.reasoning && (
        <div className="ac-reasoning">
          <div className="ac-reasoning-label">Thinking</div>
          <div className="ac-reasoning-text">{msg.reasoning}</div>
        </div>
      )}

      {msg.refusal && <div className="ac-refusal">{msg.refusal}</div>}

      {msg.content && (
        <div>
          <RichContent content={msg.content} />
        </div>
      )}

      {msg.tool_calls && msg.tool_calls.length > 0 && (
        <div className="ac-tool-calls">
          {msg.tool_calls.map((tc, i) => (
            <ToolCallCard key={i} call={tc} />
          ))}
        </div>
      )}
    </div>
  );
}

type AssistantResponseChunkLike = AgentCompletionsResponseStreamingAssistantResponseChunk;

function ToolCallCard({ call }: { call: { id?: string | null; function?: { name?: string | null; arguments?: string | null } | null } }) {
  const fn = call.function;
  let formattedArgs = fn?.arguments ?? "";
  try {
    formattedArgs = JSON.stringify(JSON.parse(formattedArgs), null, 2);
  } catch { /* keep raw */ }

  return (
    <div className="ac-tool-call">
      <div className="ac-tool-call-name">{fn?.name ?? "unknown"}</div>
      {formattedArgs && <div className="ac-tool-call-args">{formattedArgs}</div>}
    </div>
  );
}

function ToolResultBubble({ msg }: { msg: AgentCompletionsResponseToolResponse }) {
  const raw = (msg as unknown as { content: unknown }).content;
  const content =
    typeof raw === "string"
      ? raw
      : Array.isArray(raw)
        ? raw.map((p: unknown) => (typeof p === "object" && p && "text" in p ? (p as { text: string }).text : "")).join("")
        : JSON.stringify(raw);
  return (
    <div className="ac-tool-result-bubble">
      <div className="ac-tool-result-id">{msg.tool_call_id}</div>
      <div className="ac-tool-result-content">{content}</div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Usage footer
// ---------------------------------------------------------------------------

function UsageFooter({ usage }: { usage: AgentCompletionsResponseUsage }) {
  return (
    <div className="ac-footer">
      <div className="ac-footer-item">
        <span className="ac-footer-label">Prompt:</span>
        <span>{usage.prompt_tokens}</span>
      </div>
      <div className="ac-footer-item">
        <span className="ac-footer-label">Completion:</span>
        <span>{usage.completion_tokens}</span>
      </div>
      <div className="ac-footer-item">
        <span className="ac-footer-label">Total:</span>
        <span>{usage.total_tokens}</span>
      </div>
      {usage.cost !== undefined && usage.cost !== 0 && (
        <div className="ac-footer-item">
          <span className="ac-footer-label">Cost:</span>
          <span>${typeof usage.cost === "number" ? usage.cost.toFixed(6) : usage.cost}</span>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main component
// ---------------------------------------------------------------------------

export function AgentCompletionView({ entry }: { entry: AgentCompletionEntry }) {
  const chunk = entry.chunk;

  // Extract model/upstream from first assistant message in chunk
  let model = "";
  let upstream = "";
  if (chunk) {
    for (const msg of chunk.messages) {
      if (msg.role === "assistant") {
        const asst = msg as AssistantResponseChunkLike;
        model = asst.model ?? "";
        break;
      }
    }
    upstream = chunk.upstream ?? "";
  }

  // Determine status
  const hasFinish = chunk?.messages.some(
    (m) => m.role === "assistant" && (m as AssistantResponseChunkLike).finish_reason
  );
  const status = entry.error ? "error" : hasFinish ? "complete" : "streaming";

  // Format timestamp
  const created = chunk?.created;
  const timeStr = created ? new Date(created * 1000).toLocaleTimeString() : "";

  return (
    <div className="ac-container">
      {/* Header */}
      <div className="ac-header">
        <div className={`ac-status ac-status-${status}`} />
        {model && <span className="ac-header-model">{model}</span>}
        {upstream && <span className="ac-header-upstream">{upstream}</span>}
        {timeStr && <span>{timeStr}</span>}
        <span className="ac-header-id">{entry.id.slice(0, 12)}</span>
      </div>

      {/* Messages */}
      <div className="ac-messages">
        {/* Request messages (user/developer/system input) */}
        {entry.request.messages?.map((msg, i) => {
          const role = (msg as { role: string }).role;
          if (role === "developer" || role === "system") {
            return (
              <SystemBanner
                key={`req-${i}`}
                role={role}
                content={(msg as { content: unknown }).content}
              />
            );
          }
          if (role === "user") {
            return <UserBubble key={`req-${i}`} content={(msg as { content: unknown }).content} />;
          }
          if (role === "assistant") {
            return <AssistantBubble key={`req-${i}`} msg={msg as AssistantResponseChunkLike} />;
          }
          return null;
        })}

        {/* Response messages (assistant/tool output) */}
        {chunk?.messages.map((msg, i) => {
          if (msg.role === "assistant") {
            return <AssistantBubble key={`resp-${i}`} msg={msg as AssistantResponseChunkLike} />;
          }
          if (msg.role === "tool") {
            return <ToolResultBubble key={`resp-${i}`} msg={msg as AgentCompletionsResponseToolResponse} />;
          }
          return null;
        })}

        {/* Waiting state */}
        {!chunk && !entry.error && (
          <div style={{ color: "#999", fontStyle: "italic" }}>Waiting for response...</div>
        )}
      </div>

      {/* Error banner */}
      {entry.error && (
        <div className="ac-error-banner">
          Error {entry.error.code}: {JSON.stringify(entry.error.message)}
        </div>
      )}

      {/* Usage footer */}
      {chunk?.usage && <UsageFooter usage={chunk.usage} />}
    </div>
  );
}
