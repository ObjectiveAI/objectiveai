// ---------------------------------------------------------------------------
// Public types for @objectiveai/function-tree
// ---------------------------------------------------------------------------

// -- Tree Node Types --------------------------------------------------------

export type TreeNodeKind = "function" | "vector-completion" | "ensemble-llm";

export type TreeNodeState = "pending" | "streaming" | "complete" | "error";

/** Data payload for a function node (root or nested FunctionExecutionTask). */
export interface FunctionNodeData {
  kind: "function";
  functionId: string | null;
  profileId: string | null;
  output: number | number[] | null;
  taskCount: number;
  error: string | null;
  /** Swiss system round number (null if not a Swiss execution). */
  swissRound: number | null;
  /** Swiss system pool index (null if not a Swiss execution). */
  swissPoolIndex: number | null;
  /** Structural mode: "owner/repo" for nested function tasks. */
  ownerRepo?: string | null;
  /** Structural mode: function type ("scalar" or "vector"). */
  functionType?: "scalar" | "vector" | null;
  /** Truncated reasoning content from execution (root only). */
  reasoning?: string | null;
  /** Execution ID (root only). */
  executionId?: string | null;
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
  /** Structural mode: number of response options (null if expression-based). */
  responseCount?: number | null;
  /** Truncated preview of the task's prompt/question (from system/developer message). */
  promptPreview?: string | null;
  /** Full message array for detail panel display. Content is null for expression-based messages. */
  promptMessages?: Array<{ role: string; content: string | null }> | null;
}

/** Data payload for an ensemble LLM node (individual model in a task's ensemble). */
export interface EnsembleLlmNodeData {
  kind: "ensemble-llm";
  /** Readable model name (e.g., "openai/gpt-4o"). */
  model: string;
  /** Cryptic 22-char ensemble LLM ID. */
  ensembleLlmId: string;
  /** Weight of this LLM in the ensemble. */
  weight: number;
  /** Output mode used by this LLM. */
  outputMode?: string | null;
  /** Whether top_logprobs is set (enables probabilistic voting). */
  topLogprobs?: number | null;
  /** Whether this vote was from cache. */
  fromCache?: boolean;
  /** Whether this vote was from RNG. */
  fromRng?: boolean;
  /** This LLM's vote distribution over responses. */
  voteDistribution?: number[] | null;
}

export type TreeNodeData =
  | FunctionNodeData
  | VectorCompletionNodeData
  | EnsembleLlmNodeData;

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

  /**
   * Weight of the edge from this node's parent to this node.
   * Null means no weight information (draw at default thickness).
   * Values are normalized 0-1 by the tree builder.
   */
  edgeWeight: number | null;

  // Data payload
  data: TreeNodeData;
}

/** Tree rendering mode. */
export type TreeMode = "structural" | "execution";

/** Result of building a tree from execution or definition data. */
export interface TreeData {
  nodes: Map<string, TreeNode>;
  rootId: string;
  mode: TreeMode;
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
  swiss_round?: number;
  swiss_pool_index?: number;
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

// -- Structural Input Types (duck-typed function definition) ----------------

/** A single task from a function definition (before execution). */
export interface InputTaskDefinition {
  type:
    | "vector.completion"
    | "scalar.function"
    | "vector.function"
    | "placeholder.scalar.function"
    | "placeholder.vector.function";
  /** For vector.completion: the response options array. */
  responses?: unknown[];
  /** For vector.completion: the prompt messages. */
  messages?: unknown[];
  /** For function tasks: repository owner. */
  owner?: string;
  /** For function tasks: repository name. */
  repository?: string;
  /** For function tasks: commit SHA. */
  commit?: string;
  /** Skip expression (any truthy value means task can be skipped). */
  skip?: unknown;
  /** Index into input_maps for mapped execution. */
  map?: number | null;
}

/** A function definition (from function.json). */
export interface InputFunctionDefinition {
  type: "scalar.function" | "vector.function";
  description?: string;
  tasks: InputTaskDefinition[];
}

// -- Profile Input Types (duck-typed) ---------------------------------------

/** An LLM definition within a profile's ensemble. */
export interface InputProfileEnsembleLlm {
  count?: number;
  model: string;
  output_mode?: string;
  top_logprobs?: number;
  reasoning?: { enabled?: boolean };
}

/** A single task entry in a profile. */
export interface InputProfileTask {
  /** For leaf tasks with inline ensemble. */
  ensemble?: { llms: InputProfileEnsembleLlm[] };
  /** Per-LLM weights within the ensemble. */
  profile?: number[];
  /** For composite tasks referencing sub-function profiles. */
  owner?: string;
  repository?: string;
  commit?: string;
}

/** A profile definition (from profile.json). */
export interface InputProfile {
  description?: string;
  tasks: InputProfileTask[];
  /** Per-task weights. */
  profile: number[];
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
  /** Use transparent background (no canvas fill). For full-bleed layouts where the page bg shows through. */
  transparentBg: boolean;
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
  transparentBg: false,
};

// -- Node Dimensions --------------------------------------------------------

export const NODE_SIZES: Record<TreeNodeKind, { width: number; height: number }> = {
  function: { width: 220, height: 90 },
  "vector-completion": { width: 220, height: 70 },
  "ensemble-llm": { width: 160, height: 36 },
};

// -- React Component Props --------------------------------------------------

export interface FunctionTreeProps {
  /** The function execution data (streaming or complete). Null before execution. */
  data: InputFunctionExecution | null;
  /** Function definition for structural mode (renders task hierarchy before execution). */
  definition?: InputFunctionDefinition | null;
  /** Resolved sub-function definitions for recursive structural tree. Key: "owner/repo". */
  resolvedSubFunctions?: Map<string, InputFunctionDefinition>;
  /** Profile data for ensemble/LLM visualization. */
  profile?: InputProfile | null;
  /** Resolved model names: { [22-char-id]: "openai/gpt-4o" }. */
  modelNames?: Record<string, string>;
  /** Response labels per task: { [taskPath]: ["Option A", "Option B", ...] }. */
  responseLabels?: Record<string, string[]>;
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
  /** Remove border and border-radius for full-bleed canvas layouts. */
  borderless?: boolean;
}

// -- Score Colors -----------------------------------------------------------

export const SCORE_COLORS = {
  high: "#f59e0b",     // bright amber — strong
  midHigh: "#d97706",  // copper — moderate-strong
  midLow: "#b45309",   // warm stone — moderate-weak
  low: "#92400e",      // dark amber — weak
  error: "#b91c1c",    // warm brick — errors only
} as const;

export function scoreColor(score: number): string {
  if (score >= 0.5) return SCORE_COLORS.high;
  if (score >= 0.3) return SCORE_COLORS.midHigh;
  if (score >= 0.15) return SCORE_COLORS.midLow;
  return SCORE_COLORS.low;
}
