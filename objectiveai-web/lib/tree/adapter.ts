import type { FunctionDefinition, TaskDefinition } from "@/lib/functions/types";
import type { ProfileMeta } from "@/lib/profiles/types";
import type {
  InputFunctionDefinition,
  InputTaskDefinition,
  InputProfile,
  InputProfileTask,
} from "@objectiveai/function-tree";

/** Normalize type strings like "alpha.scalar.function" → "scalar.function" */
function normalizeDefType(type: string): "scalar.function" | "vector.function" {
  if (type.includes("vector")) return "vector.function";
  return "scalar.function";
}

/** Normalize task type strings to the package's expected union */
function normalizeTaskType(type: string): InputTaskDefinition["type"] {
  if (type === "vector.completion") return "vector.completion";
  if (type.startsWith("placeholder.")) {
    if (type.includes("vector")) return "placeholder.vector.function";
    return "placeholder.scalar.function";
  }
  if (type.includes("vector")) return "vector.function";
  return "scalar.function";
}

function adaptTask(task: TaskDefinition): InputTaskDefinition {
  return {
    type: normalizeTaskType(task.type),
    responses: task.responses,
    messages: Array.isArray(task.messages) ? task.messages : undefined,
    owner: task.owner,
    repository: task.repository ?? task.name,
    commit: task.commit,
    skip: task.skip,
    map: typeof task.map === "number" ? task.map : null,
  };
}

/** Convert a web-2 FunctionDefinition to the package's InputFunctionDefinition */
export function adaptDefinition(def: FunctionDefinition): InputFunctionDefinition {
  return {
    type: normalizeDefType(def.type),
    description: def.description,
    tasks: def.tasks.map(adaptTask),
  };
}

/** Convert the recursive DefMap to the package's resolvedSubFunctions Map */
export function adaptSubFunctions(
  allDefs: Map<string, FunctionDefinition>
): Map<string, InputFunctionDefinition> {
  const out = new Map<string, InputFunctionDefinition>();
  for (const [key, def] of allDefs) {
    out.set(key, adaptDefinition(def));
  }
  return out;
}

/** Convert a ProfileMeta to InputProfile, replicating ensemble for each task (auto profiles) */
export function adaptProfile(meta: ProfileMeta, taskCount: number): InputProfile {
  if (meta.kind === "tasks" && meta.taskConfigs.length > 0) {
    return {
      description: meta.description,
      profile: meta.taskWeights,
      tasks: meta.taskConfigs.map((tc): InputProfileTask => ({
        ensemble: {
          llms: tc.llms.map((l) => ({
            model: l.model,
            output_mode: l.outputMode,
            top_logprobs: l.topLogprobs ?? undefined,
            count: l.count,
          })),
        },
        profile: tc.weights,
      })),
    };
  }

  // Auto profile: same ensemble replicated for each task
  const ensembleLlms = meta.llms.map((l) => ({
    model: l.model,
    output_mode: l.outputMode,
    top_logprobs: l.topLogprobs ?? undefined,
    count: l.count,
  }));

  const taskEntry: InputProfileTask = {
    ensemble: { llms: ensembleLlms },
    profile: meta.weights,
  };

  // Equal task weights if not specified
  const taskWeights = taskCount > 0
    ? Array.from({ length: taskCount }, () => 1 / taskCount)
    : [];

  return {
    description: meta.description,
    profile: taskWeights,
    tasks: Array.from({ length: taskCount }, () => taskEntry),
  };
}
