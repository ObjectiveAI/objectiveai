import * as Tabs from "@radix-ui/react-tabs";
import type {
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
  LaboratoriesExecutionsResponseStreamingBuilderChunk,
  LaboratoriesExecutionsResponseStreamingEvaluationChunk,
} from "objectiveai";
import { AgentCompletionChat } from "../shared/AgentCompletionChat";

interface LaboratoryExecutionEntry {
  kind: "laboratory";
  id: string;
  request: { id: string };
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk | null;
  error: { id: string; code: number; message: unknown } | null;
}

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
        <div className="max-w-[800px] mx-auto -mt-4 mb-6 px-4 py-2 bg-ground-surface border border-node-border rounded-b-md font-mono text-xs text-copper-mid">
          <span className="text-info-dim mr-2">output:</span>
          {JSON.stringify(evaluation.output)}
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
    complete: "bg-green-500",
    error: "bg-error",
  }[status];

  return (
    <div className="max-w-[800px] mx-auto mb-6">
      <div className="flex items-center gap-2.5 px-4 py-2.5 bg-ground-surface border border-node-border rounded-t-md text-xs text-info-dim">
        <div className={`w-2 h-2 rounded-full shrink-0 ${statusColor} ${status === "streaming" ? "animate-pulse" : ""}`} />
        <span className="font-semibold text-info-bright">Laboratory</span>
        <span className="font-mono text-[11px] opacity-60 ml-auto">{entry.id.slice(0, 12)}</span>
      </div>

      {!chunk && !topError && (
        <div className="border border-t-0 border-node-border rounded-b-md p-4 text-info-dim italic text-center">
          Waiting for laboratory…
        </div>
      )}

      {chunk && (
        <Tabs.Root defaultValue="builders" className="border border-t-0 border-node-border rounded-b-md overflow-hidden">
          <Tabs.List className="flex border-b border-node-border bg-ground-raised">
            <Tabs.Trigger
              value="builders"
              className="flex-1 px-4 py-2 text-xs font-mono text-info-dim data-[state=active]:text-copper-mid data-[state=active]:border-b-2 data-[state=active]:border-copper-mid transition-colors"
            >
              builders ({builders.length})
            </Tabs.Trigger>
            <Tabs.Trigger
              value="evaluations"
              className="flex-1 px-4 py-2 text-xs font-mono text-info-dim data-[state=active]:text-copper-mid data-[state=active]:border-b-2 data-[state=active]:border-copper-mid transition-colors"
            >
              evaluations ({evaluations.length})
            </Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="builders" className="py-4">
            {builders.length === 0 && (
              <div className="text-info-dim italic text-center text-xs py-4">No builders yet…</div>
            )}
            {builders.map((b) => (
              <BuilderCard key={b.id || `builder-${b.index}`} builder={b} />
            ))}
          </Tabs.Content>

          <Tabs.Content value="evaluations" className="py-4">
            {evaluations.length === 0 && (
              <div className="text-info-dim italic text-center text-xs py-4">No evaluations yet…</div>
            )}
            {evaluations.map((e) => (
              <EvaluationCard key={e.id || `eval-${e.index}`} evaluation={e} />
            ))}
          </Tabs.Content>
        </Tabs.Root>
      )}

      {topError && (
        <div className="bg-error/10 border border-error/30 rounded-md px-4 py-2 text-error text-xs mt-2">
          Error {topError.code}: {JSON.stringify(topError.message)}
        </div>
      )}
    </div>
  );
}
