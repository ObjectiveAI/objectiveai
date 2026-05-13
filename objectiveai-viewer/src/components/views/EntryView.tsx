import { AgentCompletionView } from "../../AgentCompletionView";
import { FunctionInventionRecursiveView } from "../../FunctionInventionRecursiveView";
import { FunctionExecutionView } from "../../FunctionExecutionView";
import { LaboratoryExecutionView } from "./LaboratoryExecutionView";
import { ErrorBoundary } from "../shared/ErrorBoundary";
import type { Entry } from "../../types";

function EntryContent({ entry }: { entry: Entry }) {
  if (entry.kind === "agent-completion") {
    return <AgentCompletionView entry={entry} />;
  }
  if (entry.kind === "invention") {
    return <FunctionInventionRecursiveView entry={entry} />;
  }
  if (entry.kind === "execution") {
    return <FunctionExecutionView entry={entry} />;
  }
  if (entry.kind === "laboratory") {
    return <LaboratoryExecutionView entry={entry} />;
  }
  return null;
}

export function EntryView({ entry }: { entry: Entry }) {
  return (
    <ErrorBoundary>
      <EntryContent entry={entry} />
    </ErrorBoundary>
  );
}
