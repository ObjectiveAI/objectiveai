import { AgentCompletionView } from "../../AgentCompletionView";
import { FunctionInventionRecursiveView } from "../../FunctionInventionRecursiveView";
import { FunctionExecutionView } from "../../FunctionExecutionView";
import { LaboratoryExecutionView } from "./LaboratoryExecutionView";
import type { Entry } from "../../types";

export function EntryView({ entry }: { entry: Entry }) {
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
