"use client";

import styles from "./AgentChat.module.css";

/* ── Types ── */

interface ToolCall {
  id?: string;
  function: { name: string; arguments: string };
}

export type ChatMessage =
  | { role: "system"; content: string }
  | { role: "developer"; content: string }
  | { role: "user"; content: string }
  | {
      role: "assistant";
      content: string;
      model?: string;
      reasoning?: string;
      finish_reason?: string;
      tool_calls?: ToolCall[];
    }
  | { role: "tool"; tool_call_id: string; content: string };

export interface AgentChatProps {
  messages: ChatMessage[];
  model?: string;
  status: "streaming" | "complete" | "error";
  error?: string;
  label?: string;
}

/* ── Component ── */

export function AgentChat({ messages, model, status, error, label }: AgentChatProps) {
  const dotColor =
    status === "complete"
      ? "var(--copper-hot)"
      : status === "streaming"
        ? "var(--copper-mid)"
        : "var(--error)";

  return (
    <div className={styles.container}>
      {label && <div className={styles.label}>{label}</div>}
      <div className={styles.header}>
        <span className={styles.dot} style={{ background: dotColor }} />
        {model && <span className={styles.model}>{model}</span>}
        <span className={styles.status}>{status}</span>
      </div>
      <div className={styles.messages}>
        {messages.map((msg, i) => {
          if (msg.role === "system" || msg.role === "developer") {
            return <SystemBanner key={i} role={msg.role} content={msg.content} />;
          }
          if (msg.role === "user") {
            return <UserBubble key={i} content={msg.content} />;
          }
          if (msg.role === "assistant") {
            return <AssistantBubble key={i} msg={msg} />;
          }
          if (msg.role === "tool") {
            return (
              <ToolResultBubble key={i} id={msg.tool_call_id} content={msg.content} />
            );
          }
          return null;
        })}
        {status === "streaming" && <span className={styles.cursor} />}
      </div>
      {error && <div className={styles.errorBanner}>{error}</div>}
    </div>
  );
}

/* ── Sub-components ── */

function SystemBanner({ role, content }: { role: string; content: string }) {
  return (
    <div className={styles.systemBanner}>
      <span className={styles.systemRole}>{role}</span>
      <span className={styles.systemContent}>{content}</span>
    </div>
  );
}

function UserBubble({ content }: { content: string }) {
  return (
    <div className={styles.userBubble}>
      <span className={styles.bubbleRole}>user</span>
      <span className={styles.bubbleText}>{content}</span>
    </div>
  );
}

function AssistantBubble({
  msg,
}: {
  msg: Extract<ChatMessage, { role: "assistant" }>;
}) {
  return (
    <div className={styles.assistantBubble}>
      <div className={styles.assistantMeta}>
        <span className={styles.bubbleRole}>assistant</span>
        {msg.model && <span>{msg.model}</span>}
        {msg.finish_reason && (
          <span
            className={`${styles.finishBadge} ${styles[`finish_${msg.finish_reason}`] ?? ""}`}
          >
            {msg.finish_reason}
          </span>
        )}
      </div>
      {msg.reasoning && (
        <div className={styles.reasoning}>
          <span className={styles.reasoningLabel}>thinking</span>
          <span className={styles.reasoningText}>{msg.reasoning}</span>
        </div>
      )}
      {msg.content && <div className={styles.bubbleText}>{msg.content}</div>}
      {msg.tool_calls?.map((tc, i) => (
        <div key={i} className={styles.toolCall}>
          <span className={styles.toolName}>{tc.function.name}</span>
          <pre className={styles.toolArgs}>{tryFormatJson(tc.function.arguments)}</pre>
        </div>
      ))}
    </div>
  );
}

function ToolResultBubble({ id, content }: { id: string; content: string }) {
  return (
    <div className={styles.toolResult}>
      <span className={styles.toolResultId}>{id}</span>
      <pre className={styles.toolResultContent}>{content}</pre>
    </div>
  );
}

function tryFormatJson(s: string): string {
  try {
    return JSON.stringify(JSON.parse(s), null, 2);
  } catch {
    return s;
  }
}
