import { useCallback, useState, type KeyboardEvent } from "react";
import cn from "classnames";
import { agentsMessageExecute } from "@objectiveai/sdk";
import { instanceSelector } from "../lib/aih";
import { reportError } from "../lib/errors";
import { daemonExecutor } from "../lib/executor";

/**
 * The per-agent chat: a composer (Enter = send, Shift+Enter =
 * newline, Send button) plus the queued area directly above it.
 * Every send runs `agents message` through the shared SSE
 * executor; the message shows as a queued entry (newest at the
 * bottom) until its command resolves — which, with the CLI's
 * always-enqueue FIFO, means exactly "my queue row was consumed",
 * after which the real message arrives in the conversation through
 * the instance listener. Sends fire in parallel — enqueue order (=
 * send order, backend-guaranteed delivery order) is fixed the moment
 * each command lands, so the composer never locks while sends are
 * pending. Failures stay in the list with the error styling (the
 * typed text isn't lost) and a × to dismiss, and go to the error
 * toast. Text only for now — attachments are a later chunk.
 */

/** One optimistic entry in the queued area: visible from send until
 * its `agents message` command resolves (or, failed, until
 * dismissed). */
interface QueuedMessage {
  key: number;
  text: string;
  failed: boolean;
}

let nextKey = 0;

export function AgentChat({ hierarchy }: { hierarchy: string }) {
  const [queued, setQueued] = useState<QueuedMessage[]>([]);
  const [text, setText] = useState("");

  const send = useCallback(() => {
    const trimmed = text.trim();
    if (!trimmed) return;
    // Clear immediately and stay enabled — parallel pending sends are
    // the point; the backend FIFO keeps them in send order.
    setText("");
    const key = nextKey++;
    setQueued((q) => [...q, { key, text: trimmed, failed: false }]);
    void (async () => {
      try {
        const executor = await daemonExecutor();
        const result = await agentsMessageExecute(executor, {
          agent: instanceSelector(hierarchy),
          message: { Simple: trimmed },
        });
        if (result.type === "error") {
          throw new Error(
            typeof result.message === "string"
              ? result.message
              : JSON.stringify(result.message),
          );
        }
        setQueued((q) => q.filter((m) => m.key !== key));
      } catch (error) {
        reportError(`agent ${hierarchy} message`, error);
        setQueued((q) =>
          q.map((m) => (m.key === key ? { ...m, failed: true } : m)),
        );
      }
    })();
  }, [text, hierarchy]);

  const dismiss = useCallback((key: number) => {
    setQueued((q) => q.filter((m) => m.key !== key));
  }, []);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        send();
      }
    },
    [send],
  );

  return (
    <div className={cn("max-w-content", "w-full", "mx-auto", "mb-4", "px-4")}>
      {queued.length > 0 && (
        <div className={cn("flex", "flex-col", "gap-1.5", "mb-2")}>
          {queued.map((m) => (
            <div
              key={m.key}
              className={cn(
                "flex",
                "items-start",
                "gap-2",
                "bg-ground-surface",
                "border",
                "rounded-sm",
                "px-2.5",
                "py-1.5",
                m.failed ? "border-error" : "border-node-border",
              )}
            >
              <span
                className={cn(
                  "shrink-0",
                  "px-1.5",
                  "py-px",
                  "rounded-sm",
                  "border",
                  "font-mono",
                  "text-xs",
                  m.failed
                    ? ["border-error/70", "bg-error/10", "text-error"]
                    : [
                        "border-copper-mid/70",
                        "bg-copper-warm/10",
                        "text-copper-bright",
                      ],
                )}
              >
                {m.failed ? "failed" : "queued"}
              </span>
              <div
                className={cn(
                  "flex-1",
                  "min-w-0",
                  "text-sm",
                  "font-mono",
                  "whitespace-pre-wrap",
                  "break-words",
                  m.failed ? "text-error" : "text-info-mid",
                )}
              >
                {m.text}
              </div>
              {m.failed && (
                <button
                  className={cn(
                    "shrink-0",
                    "font-mono",
                    "text-xs",
                    "text-error",
                    "hover:text-info-bright",
                    "transition-colors",
                  )}
                  onClick={() => dismiss(m.key)}
                  aria-label="Dismiss failed message"
                >
                  ×
                </button>
              )}
            </div>
          ))}
        </div>
      )}
      <div className={cn("flex", "gap-2")}>
        <textarea
          className={cn(
            "flex-1",
            "bg-ground-surface",
            "border",
            "border-node-border",
            "rounded-md",
            "px-3",
            "py-2",
            "text-sm",
            "text-info-bright",
            "font-mono",
            "resize-none",
            "placeholder:text-info-dim",
            "focus:outline-none",
            "focus:border-copper-dim",
          )}
          rows={1}
          placeholder="Message agent…"
          aria-label="Send a message to this agent"
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={handleKeyDown}
        />
        <button
          className={cn(
            "px-3",
            "py-2",
            "bg-copper-warm",
            "text-ground",
            "font-mono",
            "text-xs",
            "font-semibold",
            "rounded-md",
            "hover:bg-copper-mid",
            "disabled:opacity-40",
            "disabled:cursor-not-allowed",
            "transition-colors",
          )}
          onClick={send}
          disabled={!text.trim()}
        >
          Send
        </button>
      </div>
    </div>
  );
}
