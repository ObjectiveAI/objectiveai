import { useState, useCallback, type KeyboardEvent } from "react";
import cn from "classnames";
import { tauriInvoke } from "../../lib/tauri";

export function MessageInput({ responseId }: { responseId: string }) {
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);

  const send = useCallback(async () => {
    const trimmed = text.trim();
    if (!trimmed || sending) return;
    setSending(true);
    try {
      await tauriInvoke("notify_agent_completion", {
        params: {
          response_id: responseId,
          content: trimmed,
        },
      });
      setText("");
    } catch (err) {
      console.error("notify_agent_completion failed:", err);
    } finally {
      setSending(false);
    }
  }, [text, sending, responseId]);

  const handleKeyDown = useCallback((e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }, [send]);

  return (
    <div className={cn("max-w-content", "mx-auto", "mb-6", "px-4")}>
      <div className={cn("text-[10px]", "font-mono", "text-info-dim", "uppercase", "tracking-wide", "mb-1.5", "px-1")}>
        Multi-turn — send a follow-up while the agent is running
      </div>
      <div className={cn("flex", "gap-2")}>
        <textarea
          className={cn("flex-1", "bg-ground-surface", "border", "border-node-border", "rounded-md", "px-3", "py-2", "text-sm", "text-info-bright", "font-mono", "resize-none", "placeholder:text-info-dim", "focus:outline-none", "focus:border-copper-dim")}
          rows={1}
          placeholder="Message agent…"
          aria-label="Send follow-up message to running agent"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={sending}
        />
        <button
          className={cn("px-3", "py-2", "bg-copper-warm", "text-ground", "font-mono", "text-xs", "font-semibold", "rounded-md", "hover:bg-copper-mid", "disabled:opacity-40", "disabled:cursor-not-allowed", "transition-colors")}
          onClick={send}
          disabled={!text.trim() || sending}
        >
          Send
        </button>
      </div>
    </div>
  );
}
