// ---------------------------------------------------------------------------
// Public types for @objectiveai/function-tree
// ---------------------------------------------------------------------------

// -- Tree Node Types --------------------------------------------------------

export type TreeNodeKind = "function" | "vector-completion" | "llm";

export type TreeNodeState = "pending" | "streaming" | "complete" | "error";

/** Data payload for a function node (root or nested FunctionExecutionTask). */
export interface FunctionNodeData {
  kind: "function";
  functionId: string | null;
  profileId: string | null;
  output: number | number[] | null;
  taskCount: number;
  error: string | null;
}

/** Data payload for a vector completion task node. */
export interface VectorCompletionNodeData {
  kind: "vector-completion";
  taskIndex: number;
  taskPath: number[];
  scores: number[] | null;
  responses: string[] | null;
  voteCount: number;
  /** Raw vote data for DetailPanel display (LLM nodes no longer rendered in tree). */
  votes: InputVote[] | null;
  /** Raw completion data for DetailPanel display. */
  completions: InputCompletion[] | null;
  error: string | null;
}

/** Data payload for an LLM leaf node (one Vote). */
export interface LlmNodeData {
  kind: "llm";
  /** 22-char content-addressed Ensemble LLM ID. */
  modelId: string;
  /** Resolved readable name (e.g., "openai/gpt-4o"). Null until resolved. */
  modelName: string | null;
  /** Vote distribution over responses. Null while pending. */
  vote: number[] | null;
  /** Weight assigned to this vote. */
  weight: number;
  /** Accumulated streaming completion text. */
  streamingText: string;
  /** Whether this vote came from the global cache. */
  fromCache: boolean;
  /** Whether this vote was generated via RNG. */
  fromRng: boolean;
  /** Index into the ensemble. */
  flatEnsembleIndex: number;
}

export type TreeNodeData =
  | FunctionNodeData
  | VectorCompletionNodeData
  | LlmNodeData;

/** A single node in the function execution tree. */
export interface TreeNode {
  id: string;
  kind: TreeNodeKind;
  label: string;
  parentId: string | null;
  children: string[];

  // Layout (computed by layout algorithm, default 0)
  x: number;
  y: number;
  width: number;
  height: number;

  // Visual state
  state: TreeNodeState;

  // Data payload
  data: TreeNodeData;
}

/** Result of building a tree from execution data. */
export interface TreeData {
  nodes: Map<string, TreeNode>;
  rootId: string;
}

// -- Input Data Types (duck-typed, no SDK import) ---------------------------

/** Structurally compatible with SDK's Vote. */
export interface InputVote {
  model: string;
  ensemble_index?: number;
  flat_ensemble_index?: number;
  vote: number[];
  weight: number;
  retry?: boolean;
  from_cache?: boolean;
  from_rng?: boolean;
}

/** Structurally compatible with SDK's ChatCompletion choice. */
export interface InputCompletionChoice {
  delta?: { content?: string };
  message?: { content?: string };
}

/** Structurally compatible with SDK's ChatCompletion. */
export interface InputCompletion {
  model: string;
  choices?: InputCompletionChoice[];
}

/** Structurally compatible with a VectorCompletionTask. */
export interface InputVectorCompletionTask {
  index?: number;
  task_index?: number;
  task_path?: number[];
  votes?: InputVote[];
  completions?: InputCompletion[];
  scores?: number[];
  error?: { message?: string } | null;
}

/** Structurally compatible with a FunctionExecutionTask. */
export interface InputFunctionExecutionTask {
  index?: number;
  task_index?: number;
  task_path?: number[];
  tasks: InputTask[];
  output?: number | number[];
  error?: { message?: string } | null;
  function?: string | null;
  profile?: string | null;
}

export type InputTask =
  | InputVectorCompletionTask
  | InputFunctionExecutionTask;

/** Structurally compatible with SDK's FunctionExecution. */
export interface InputFunctionExecution {
  id?: string;
  tasks?: InputTask[];
  output?: number | number[];
  error?: { message?: string } | null;
  function?: string | null;
  profile?: string | null;
  reasoning?: {
    choices?: Array<{ message?: { content?: string } }>;
  } | null;
}

// -- Configuration ----------------------------------------------------------

export interface FunctionTreeConfig {
  /** Tree orientation. Default: "vertical" (root at top). */
  orientation: "vertical" | "horizontal";
  /** Horizontal spacing between sibling nodes in pixels. */
  nodeGapX: number;
  /** Vertical spacing between tree levels in pixels. */
  nodeGapY: number;
  /** Whether to animate transitions when data changes. */
  animate: boolean;
  /** Animation duration in ms. */
  animationDuration: number;
  /** Minimum zoom level. */
  minZoom: number;
  /** Maximum zoom level. */
  maxZoom: number;
  /** Color theme. "auto" reads from CSS/prefers-color-scheme. */
  theme: "light" | "dark" | "auto";
  /** Max children before switching to grid layout. Default: 20. */
  gridThreshold: number;
}

export const DEFAULT_CONFIG: FunctionTreeConfig = {
  orientation: "vertical",
  nodeGapX: 24,
  nodeGapY: 80,
  animate: true,
  animationDuration: 300,
  minZoom: 0.02,
  maxZoom: 3,
  theme: "auto",
  gridThreshold: 20,
};

// -- Node Dimensions --------------------------------------------------------

export const NODE_SIZES: Record<TreeNodeKind, { width: number; height: number }> = {
  function: { width: 200, height: 80 },
  "vector-completion": { width: 180, height: 70 },
  llm: { width: 150, height: 60 },
};

// -- React Component Props --------------------------------------------------

export interface FunctionTreeProps {
  /** The function execution data (streaming or complete). Null before execution. */
  data: InputFunctionExecution | null;
  /** Resolved model names: { [22-char-id]: "openai/gpt-4o" }. */
  modelNames?: Record<string, string>;
  /** Configuration overrides. */
  config?: Partial<FunctionTreeConfig>;
  /** Called when a node is clicked. */
  onNodeClick?: (node: TreeNode) => void;
  /** Called when a node is hovered. */
  onNodeHover?: (node: TreeNode | null) => void;
  /** Width (CSS value). Default: "100%". */
  width?: number | string;
  /** Height (CSS value). Default: 400. */
  height?: number | string;
  /** CSS class name for the container. */
  className?: string;
}

// -- Score Colors -----------------------------------------------------------

export const SCORE_COLORS = {
  green: "rgb(34, 197, 94)",
  yellow: "rgb(234, 179, 8)",
  orange: "rgb(249, 115, 22)",
  red: "rgb(239, 68, 68)",
} as const;

export function scoreColor(score: number): string {
  if (score >= 0.66) return SCORE_COLORS.green;
  if (score >= 0.33) return SCORE_COLORS.yellow;
  if (score >= 0.15) return SCORE_COLORS.orange;
  return SCORE_COLORS.red;
}
