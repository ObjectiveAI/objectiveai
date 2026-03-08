import type {
  TreeNode,
  TreeData,
  TreeNodeState,
  InputFunctionExecution,
  InputTask,
  InputVectorCompletionTask,
  InputFunctionExecutionTask,
} from "../types";
import { NODE_SIZES as SIZES } from "../types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Discriminate between vector completion and function execution tasks. */
function isVectorCompletionTask(
  task: InputTask
): task is InputVectorCompletionTask {
  return "votes" in task || "completions" in task || "scores" in task;
}

function isFunctionExecutionTask(
  task: InputTask
): task is InputFunctionExecutionTask {
  return "tasks" in task && Array.isArray((task as InputFunctionExecutionTask).tasks);
}

function nodeId(prefix: string, path: number[]): string {
  return path.length > 0 ? `${prefix}-${path.join("-")}` : prefix;
}

function taskState(
  task: InputVectorCompletionTask
): TreeNodeState {
  if (task.error) return "error";
  if (task.scores && task.scores.length > 0) return "complete";
  if (
    task.completions &&
    task.completions.length > 0
  )
    return "streaming";
  return "pending";
}

function functionState(
  exec: InputFunctionExecution | InputFunctionExecutionTask
): TreeNodeState {
  if (exec.error) return "error";
  if (exec.output !== undefined && exec.output !== null) return "complete";
  if (exec.tasks && exec.tasks.length > 0) return "streaming";
  return "pending";
}

// ---------------------------------------------------------------------------
// Build Tree
// ---------------------------------------------------------------------------

/**
 * Transform a FunctionExecution (or streaming accumulation) into a flat
 * Map<string, TreeNode> with a root ID. This is the core data transform
 * that the layout algorithm and renderer consume.
 */
export function buildTree(
  execution: InputFunctionExecution | null,
  modelNames?: Record<string, string>,
  responseLabels?: Record<string, string[]>
): TreeData | null {
  if (!execution) return null;

  const nodes = new Map<string, TreeNode>();
  const rootId = "root";

  // Create root function node
  const rootNode: TreeNode = {
    id: rootId,
    kind: "function",
    label: execution.function
      ? execution.function.split("/").pop() || "Function"
      : "Function",
    parentId: null,
    children: [],
    x: 0,
    y: 0,
    width: SIZES.function.width,
    height: SIZES.function.height,
    state: functionState(execution),
    data: {
      kind: "function",
      functionId: execution.function ?? null,
      profileId: execution.profile ?? null,
      output:
        execution.output !== undefined
          ? (execution.output as number | number[])
          : null,
      taskCount: execution.tasks?.length ?? 0,
      error: execution.error?.message ?? null,
      swissRound: null,
      swissPoolIndex: null,
    },
  };
  nodes.set(rootId, rootNode);

  // Process tasks
  if (execution.tasks) {
    for (let i = 0; i < execution.tasks.length; i++) {
      processTask(execution.tasks[i], rootId, i, nodes, modelNames, responseLabels);
    }
  }

  return { nodes, rootId };
}

function processTask(
  task: InputTask,
  parentId: string,
  fallbackIndex: number,
  nodes: Map<string, TreeNode>,
  modelNames?: Record<string, string>,
  responseLabels?: Record<string, string[]>
): void {
  // A task with sub-tasks is always a function execution task,
  // regardless of whether it also carries scores/votes properties
  if (isFunctionExecutionTask(task)) {
    processFunctionTask(task, parentId, fallbackIndex, nodes, modelNames, responseLabels);
  } else {
    processVectorCompletionTask(
      task as InputVectorCompletionTask,
      parentId,
      fallbackIndex,
      nodes,
      modelNames,
      responseLabels
    );
  }
}

function processFunctionTask(
  task: InputFunctionExecutionTask,
  parentId: string,
  fallbackIndex: number,
  nodes: Map<string, TreeNode>,
  modelNames?: Record<string, string>,
  responseLabels?: Record<string, string[]>
): void {
  const idx = task.index ?? fallbackIndex;
  const path = task.task_path ?? [idx];
  const id = nodeId("func", path);

  const hasSwiss = task.swiss_round !== undefined;
  const label = hasSwiss
    ? `Round ${task.swiss_round} · Pool ${task.swiss_pool_index ?? 0}`
    : task.function
      ? task.function.split("/").pop() || `Task ${idx}`
      : `Task ${idx}`;

  const node: TreeNode = {
    id,
    kind: "function",
    label,
    parentId,
    children: [],
    x: 0,
    y: 0,
    width: SIZES.function.width,
    height: SIZES.function.height,
    state: functionState(task),
    data: {
      kind: "function",
      functionId: task.function ?? null,
      profileId: task.profile ?? null,
      output:
        task.output !== undefined
          ? (task.output as number | number[])
          : null,
      taskCount: task.tasks?.length ?? 0,
      error: task.error?.message ?? null,
      swissRound: task.swiss_round ?? null,
      swissPoolIndex: task.swiss_pool_index ?? null,
    },
  };

  nodes.set(id, node);

  // Add as child of parent
  const parent = nodes.get(parentId);
  if (parent) parent.children.push(id);

  // Recurse into sub-tasks
  if (task.tasks) {
    for (let i = 0; i < task.tasks.length; i++) {
      processTask(task.tasks[i], id, i, nodes, modelNames, responseLabels);
    }
  }
}

function processVectorCompletionTask(
  task: InputVectorCompletionTask,
  parentId: string,
  fallbackIndex: number,
  nodes: Map<string, TreeNode>,
  modelNames?: Record<string, string>,
  responseLabels?: Record<string, string[]>
): void {
  const idx = task.index ?? fallbackIndex;
  const path = task.task_path ?? [idx];
  const id = nodeId("vc", path);

  // Resolve response labels for this task by its path key
  const pathKey = path.join(",");
  const labels = responseLabels?.[pathKey] ?? null;

  const node: TreeNode = {
    id,
    kind: "vector-completion",
    label: `Task ${idx}`,
    parentId,
    children: [],
    x: 0,
    y: 0,
    width: SIZES["vector-completion"].width,
    height: SIZES["vector-completion"].height,
    state: taskState(task),
    data: {
      kind: "vector-completion",
      taskIndex: task.task_index ?? idx,
      taskPath: path,
      scores: task.scores ?? null,
      responses: labels,
      voteCount: task.votes?.length ?? 0,
      votes: task.votes ?? null,
      completions: task.completions ?? null,
      error: task.error?.message ?? null,
    },
  };

  nodes.set(id, node);

  // Add as child of parent
  const parent = nodes.get(parentId);
  if (parent) parent.children.push(id);
}
