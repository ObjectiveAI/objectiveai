import cn from "classnames";
import * as Tabs from "@radix-ui/react-tabs";
import type {
  LaboratoriesExecutionsResponseStreamingBuilderChunk,
  LaboratoriesExecutionsResponseStreamingEvaluationChunk,
} from "@objectiveai/sdk";
import { AgentCompletionChat } from "../shared/AgentCompletionChat";
import { OutputBar } from "../shared/OutputBar";
import type { LaboratoryExecutionEntry } from "../../types";

function BuilderCard({ builder }: { builder: LaboratoriesExecutionsResponseStreamingBuilderChunk }) {
  const compError = builder.error
    ? { code: builder.error.code, message: builder.error.message }
    : null;
  return (
    <AgentCompletionChat
      label={`Builder #${builder.index} (agent ${builder.agent_index})`}
      chunk={builder}
      error={compError}
      id={builder.id}
    />
  );
}

function EvaluationCard({ evaluation }: { evaluation: LaboratoriesExecutionsResponseStreamingEvaluationChunk }) {
  const compError = evaluation.error
    ? { code: evaluation.error.code, message: evaluation.error.message }
    : null;
  return (
    <div>
      <AgentCompletionChat
        label={`Evaluation #${evaluation.index} (agent ${evaluation.agent_index})`}
        chunk={evaluation}
        error={compError}
        id={evaluation.id}
      />
      {evaluation.output !== undefined && evaluation.output !== null && (
        <div className={cn("max-w-content", "mx-auto", "mb-6", "px-4", "py-2", "bg-ground-surface", "border", "border-t-0", "border-node-border", "rounded-b-md")}>
          <div className={cn("text-[10px]", "font-mono", "text-info-dim", "uppercase", "tracking-wide", "mb-1.5")}>output</div>
          <OutputBar output={evaluation.output} />
        </div>
      )}
    </div>
  );
}

export function LaboratoryExecutionView({ entry }: { entry: LaboratoryExecutionEntry }) {
  const chunk = entry.chunk;
  const topError = entry.error
    ? { code: entry.error.code, message: entry.error.message }
    : null;

  const builders = chunk?.builders ?? [];
  const evaluations = chunk?.evaluations ?? [];

  const hasFinish = chunk?.usage !== undefined && chunk?.usage !== null;
  const status = topError ? "error" : hasFinish ? "complete" : "streaming";
  const statusColor = {
    streaming: "bg-copper-hot",
    complete: "bg-success",
    error: "bg-error",
  }[status];

  return (
    <div className={cn("max-w-content", "mx-auto", "mb-6")}>
      <div className={cn("flex", "items-center", "gap-2.5", "px-4", "py-2.5", "bg-ground-surface", "border", "border-node-border", "rounded-t-md", "text-xs", "text-info-dim")}>
        <div className={cn("w-2", "h-2", "rounded-full", "shrink-0", statusColor, status === "streaming" && "animate-pulse")} />
        <span className={cn("font-semibold", "text-info-bright")}>Laboratory</span>
        <span className={cn("font-mono", "text-[10px]", "opacity-60", "ml-auto")}>{entry.id.slice(0, 12)}</span>
      </div>

      {!chunk && !topError && (
        <div className={cn("border", "border-t-0", "border-node-border", "rounded-b-md", "p-4", "text-info-dim", "italic", "text-center")}>
          Waiting for laboratory…
        </div>
      )}

      {chunk && (
        <Tabs.Root defaultValue="builders" className={cn("border", "border-t-0", "border-node-border", "rounded-b-md", "overflow-hidden")}>
          <Tabs.List className={cn("flex", "border-b", "border-node-border", "bg-ground-raised")}>
            <Tabs.Trigger
              value="builders"
              className={cn("flex-1", "px-4", "py-2", "text-xs", "font-mono", "text-info-dim", "data-[state=active]:text-copper-mid", "data-[state=active]:border-b-2", "data-[state=active]:border-copper-mid", "transition-colors")}
            >
              builders ({builders.length})
            </Tabs.Trigger>
            <Tabs.Trigger
              value="evaluations"
              className={cn("flex-1", "px-4", "py-2", "text-xs", "font-mono", "text-info-dim", "data-[state=active]:text-copper-mid", "data-[state=active]:border-b-2", "data-[state=active]:border-copper-mid", "transition-colors")}
            >
              evaluations ({evaluations.length})
            </Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="builders" className={cn("py-4")}>
            {builders.length === 0 && (
              <div className={cn("text-info-dim", "italic", "text-center", "text-xs", "py-4")}>No builders yet…</div>
            )}
            {builders.map((b) => (
              <BuilderCard key={b.id || `builder-${b.index}`} builder={b} />
            ))}
          </Tabs.Content>

          <Tabs.Content value="evaluations" className={cn("py-4")}>
            {evaluations.length === 0 && (
              <div className={cn("text-info-dim", "text-center", "text-xs", "py-4")}>
                {status === "streaming" ? (
                  <div className={cn("flex", "items-center", "justify-center", "gap-2")}>
                    <span className={cn("w-1.5", "h-1.5", "rounded-full", "bg-copper-hot", "animate-pulse")} />
                    <span>Evaluations begin after builders complete</span>
                  </div>
                ) : (
                  <span className={cn("italic")}>No evaluations configured</span>
                )}
              </div>
            )}
            {evaluations.map((e) => (
              <EvaluationCard key={e.id || `eval-${e.index}`} evaluation={e} />
            ))}
          </Tabs.Content>
        </Tabs.Root>
      )}

      {topError && (
        <div role="alert" className={cn("bg-error/10", "border", "border-error/30", "rounded-md", "px-4", "py-2", "text-error", "text-xs", "mt-2")}>
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
