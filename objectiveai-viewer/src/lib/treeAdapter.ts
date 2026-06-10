import type {
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsExecutionsResponseStreamingTaskChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk,
  FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk,
  FunctionsExecutionsResponseStreamingReasoningSummaryChunk,
  VectorCompletionsResponseStreamingAgentCompletionChunk,
  VectorCompletionsResponseVote,
} from "@objectiveai/sdk";
import type {
  InputFunctionExecution,
  InputTask,
  InputVote,
  InputCompletion,
} from "@objectiveai/function-tree";

type RemotePath = { owner?: string; repository?: string; name?: string } | null | undefined;

function remotePathToString(path: RemotePath): string | null {
  if (!path || typeof path === "string") return path ?? null;
  if ("owner" in path && path.owner && "repository" in path && path.repository) {
    return `${path.owner}/${path.repository}`;
  }
  if ("name" in path && path.name) return path.name;
  return null;
}

function adaptVote(v: VectorCompletionsResponseVote): InputVote {
  return {
    model: v.agent_full_id,
    vote: v.vote,
    weight: v.weight,
    ensemble_index: v.swarm_index,
    flat_ensemble_index: v.flat_swarm_index,
    from_cache: v.from_cache ?? undefined,
    retry: v.retry ?? undefined,
  };
}

function extractContent(
  messages: VectorCompletionsResponseStreamingAgentCompletionChunk["messages"],
): string {
  const parts: string[] = [];
  for (const msg of messages) {
    if (msg.role !== "assistant") continue;
    const c = msg.content;
    if (typeof c === "string") {
      parts.push(c);
    } else if (Array.isArray(c)) {
      for (const p of c) {
        if (typeof p === "object" && p !== null && "text" in p) {
          parts.push((p as { text: string }).text);
        }
      }
    }
  }
  return parts.join("");
}

function adaptCompletion(
  comp: VectorCompletionsResponseStreamingAgentCompletionChunk,
): InputCompletion {
  let model = "";
  for (const m of comp.messages) {
    if (m.role === "assistant" && "model" in m) {
      model = (m as { model: string }).model;
      break;
    }
  }
  return {
    model,
    choices: [{ message: { content: extractContent(comp.messages) } }],
  };
}

function adaptReasoning(
  r: FunctionsExecutionsResponseStreamingReasoningSummaryChunk | null | undefined,
): InputFunctionExecution["reasoning"] {
  if (!r) return null;
  const content = extractContent(r.messages);
  if (!content) return null;
  return { choices: [{ message: { content } }] };
}

function isVectorCompletionTask(
  t: FunctionsExecutionsResponseStreamingTaskChunk,
): t is FunctionsExecutionsResponseStreamingVectorCompletionTaskChunk {
  return (
    typeof t === "object" &&
    t !== null &&
    "object" in t &&
    (t as { object?: string }).object === "vector.completion.chunk"
  );
}

function isFunctionExecutionTask(
  t: FunctionsExecutionsResponseStreamingTaskChunk,
): t is FunctionsExecutionsResponseStreamingFunctionExecutionTaskChunk {
  if (typeof t !== "object" || t === null || !("object" in t)) return false;
  const o = (t as { object?: string }).object;
  return o === "scalar.function.execution.chunk" || o === "vector.function.execution.chunk";
}

function adaptTask(t: FunctionsExecutionsResponseStreamingTaskChunk): InputTask | null {
  if (isVectorCompletionTask(t)) {
    return {
      index: t.index,
      task_index: t.task_index,
      task_path: t.task_path,
      votes: t.votes?.map(adaptVote),
      completions: t.completions?.map(adaptCompletion),
      scores: t.scores,
      error: t.error ? { message: String(t.error.message) } : null,
    };
  }
  if (isFunctionExecutionTask(t)) {
    return {
      index: t.index,
      task_index: t.task_index,
      task_path: t.task_path,
      tasks: adaptTasks(t.tasks),
      output: adaptOutput(t.output),
      error: t.error ? { message: String(t.error.message) } : null,
      function: remotePathToString(t.function as RemotePath),
      profile: remotePathToString(t.profile as RemotePath),
      swiss_round: t.swiss_round ?? undefined,
      swiss_pool_index: t.swiss_pool_index ?? undefined,
    };
  }
  return null;
}

function adaptTasks(tasks: FunctionsExecutionsResponseStreamingTaskChunk[] | undefined): InputTask[] {
  if (!tasks) return [];
  const out: InputTask[] = [];
  for (const t of tasks) {
    const adapted = adaptTask(t);
    if (adapted) out.push(adapted);
  }
  return out;
}

function adaptOutput(
  output: { output?: number | number[] | number[][] | unknown } | null | undefined,
): number | number[] | undefined {
  if (!output) return undefined;
  const v = output.output;
  if (typeof v === "number") return v;
  if (Array.isArray(v) && v.every((x) => typeof x === "number")) return v as number[];
  return undefined;
}

export function toInputFunctionExecution(
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
): InputFunctionExecution {
  return {
    id: chunk.id,
    tasks: adaptTasks(chunk.tasks),
    output: adaptOutput(chunk.output),
    error: chunk.error ? { message: String(chunk.error.message) } : null,
    function: remotePathToString(chunk.function as RemotePath),
    profile: remotePathToString(chunk.profile as RemotePath),
    reasoning: adaptReasoning(chunk.reasoning),
  };
}
