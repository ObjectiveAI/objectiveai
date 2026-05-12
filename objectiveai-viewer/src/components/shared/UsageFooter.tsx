import type { AgentCompletionsResponseUsage } from "objectiveai";

export function UsageFooter({ usage }: { usage: AgentCompletionsResponseUsage }) {
  return (
    <div className="px-4 py-2 border-t border-node-border text-[11px] text-info-dim font-mono tabular-nums flex gap-4 flex-wrap">
      <div className="flex gap-1">
        <span className="text-copper-dim">Prompt:</span>
        <span>{usage.prompt_tokens}</span>
      </div>
      <div className="flex gap-1">
        <span className="text-copper-dim">Completion:</span>
        <span>{usage.completion_tokens}</span>
      </div>
      <div className="flex gap-1">
        <span className="text-copper-dim">Total:</span>
        <span>{usage.total_tokens}</span>
      </div>
      {usage.cost !== undefined && usage.cost !== 0 && (
        <div className="flex gap-1">
          <span className="text-copper-dim">Cost:</span>
          <span>${typeof usage.cost === "number" ? usage.cost.toFixed(6) : usage.cost}</span>
        </div>
      )}
    </div>
  );
}
