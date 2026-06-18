import type {
  TreeNode,
  TreeData,
  TreeNodeState,
  InputFunctionExecution,
  InputTask,
  InputVectorCompletionTask,
  InputFunctionExecutionTask,
  InputProfile,
  EnsembleLlmNodeData,
} from "../types";
import { NODE_SIZES as SIZES } from "../types";
import { nodeId } from "./node-id";

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

  // Extract reasoning content
  const reasoningContent = execution.reasoning?.choices?.[0]?.message?.content ?? null;
  const reasoningPreview = reasoningContent
    ? (reasoningContent.length > 60 ? reasoningContent.slice(0, 57) + "…" : reasoningContent)
    : null;

  // Root node height: taller when it has output or reasoning
  let rootHeight = execution.output !== undefined ? 106 : SIZES.function.height;
  if (reasoningPreview) rootHeight += 14;
  if (execution.id) rootHeight += 12;

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
    height: rootHeight,
    state: functionState(execution),
    edgeWeight: null,
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
      reasoning: reasoningPreview,
      executionId: execution.id ?? null,
    },
  };
  nodes.set(rootId, rootNode);

  // Process tasks
  if (execution.tasks) {
    for (let i = 0; i < execution.tasks.length; i++) {
      processTask(execution.tasks[i], rootId, i, nodes, modelNames, responseLabels);
    }
  }

  return { nodes, rootId, mode: "execution" as const };
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
    edgeWeight: null,
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

  // Dynamic height based on data density
  let vcHeight = SIZES["vector-completion"].height; // base 70
  const scores = task.scores ?? null;
  if (scores && scores.length > 0) {
    const barCount = Math.min(scores.length, 4);
    vcHeight += barCount * 12; // 8px bar + 4px gap each
    if (scores.length > 4) vcHeight += 12; // "+N more" line
  }

  const node: TreeNode = {
    id,
    kind: "vector-completion",
    label: `Task ${idx}`,
    parentId,
    children: [],
    x: 0,
    y: 0,
    width: SIZES["vector-completion"].width,
    height: vcHeight,
    state: taskState(task),
    edgeWeight: null,
    data: {
      kind: "vector-completion",
      taskIndex: task.task_index ?? idx,
      taskPath: path,
      scores,
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

  // Create ensemble LLM child nodes from votes
  if (task.votes && task.votes.length > 0) {
    // Find max weight among votes for normalization
    const maxWeight = Math.max(...task.votes.map((v) => v.weight));

    for (let v = 0; v < task.votes.length; v++) {
      const vote = task.votes[v];
      const llmId = nodeId("llm", [...path, v]);
      const modelName = modelNames?.[vote.model]
        ? modelNames[vote.model]
        : vote.model;

      const hasDistribution = vote.vote.length > 0;
      const llmHeight = hasDistribution ? 58 : SIZES["ensemble-llm"].height;

      const llmNode: TreeNode = {
        id: llmId,
        kind: "ensemble-llm",
        label: modelName.includes("/")
          ? modelName.split("/").pop() || modelName
          : modelName.length > 12
            ? modelName.slice(0, 8)
            : modelName,
        parentId: id,
        children: [],
        x: 0,
        y: 0,
        width: SIZES["ensemble-llm"].width,
        height: llmHeight,
        state: "complete",
        edgeWeight: maxWeight > 0 ? vote.weight / maxWeight : null,
        data: {
          kind: "ensemble-llm",
          model: modelName,
          ensembleLlmId: vote.model,
          weight: vote.weight,
          fromCache: vote.from_cache ?? false,
          fromRng: vote.from_rng ?? false,
          voteDistribution: hasDistribution ? vote.vote : null,
        } satisfies EnsembleLlmNodeData,
      };

      nodes.set(llmId, llmNode);
      node.children.push(llmId);
    }
  }
}

// ---------------------------------------------------------------------------
// Apply Profile Weights
// ---------------------------------------------------------------------------

/**
 * Apply profile weights to tree node edges. Mutates nodes in place.
 *
 * Profile structure:
 * - `profile.profile[]` — per-task weights for the root function's children
 * - `profile.tasks[i].profile[]` — per-LLM weights within each task's ensemble
 *
 * Edge weights are normalized to [0, 1] relative to the maximum weight
 * among siblings so the renderer can map to the 1-2px thickness range.
 */
export function applyProfileWeights(
  treeData: TreeData,
  profile: InputProfile | null | undefined,
): void {
  if (!profile) return;

  const { nodes, rootId } = treeData;
  const root = nodes.get(rootId);
  if (!root) return;

  // Apply per-task weights to direct children of root
  const taskWeights = profile.profile;
  if (taskWeights && taskWeights.length > 0) {
    applyWeightsToChildren(nodes, root, taskWeights);
  }

  // Apply per-LLM weights from profile.tasks[i].profile
  // to ensemble-llm children of each vector-completion node
  if (profile.tasks) {
    for (let i = 0; i < root.children.length && i < profile.tasks.length; i++) {
      const childId = root.children[i];
      const child = nodes.get(childId);
      if (!child) continue;

      const profileTask = profile.tasks[i];
      if (profileTask.profile && profileTask.profile.length > 0) {
        applyWeightsToChildren(nodes, child, profileTask.profile);
      }
    }
  }
}

/**
 * Apply a weight array to the children of a node, normalizing to [0, 1].
 */
function applyWeightsToChildren(
  nodes: Map<string, TreeNode>,
  parent: TreeNode,
  weights: number[],
): void {
  const maxWeight = Math.max(...weights);
  if (maxWeight <= 0) return;

  for (let i = 0; i < parent.children.length && i < weights.length; i++) {
    const child = nodes.get(parent.children[i]);
    if (!child) continue;
    child.edgeWeight = weights[i] / maxWeight;
  }
}
