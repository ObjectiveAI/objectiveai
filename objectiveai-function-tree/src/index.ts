// ---------------------------------------------------------------------------
// @objectiveai/function-tree — Public API
// ---------------------------------------------------------------------------

// React component
export { FunctionTree } from "./react/FunctionTree";

// Core engine (for framework-agnostic usage)
export { FunctionTreeEngine } from "./core/engine";

// Data transformation
export { buildTree } from "./core/tree-data";

// Layout
export { layoutTree, treeBounds } from "./core/layout";

// Viewport
export { Viewport } from "./core/viewport";

// Types
export type {
  TreeNode,
  TreeNodeKind,
  TreeNodeState,
  TreeNodeData,
  TreeData,
  FunctionNodeData,
  VectorCompletionNodeData,
  LlmNodeData,
  FunctionTreeConfig,
  FunctionTreeProps,
  InputFunctionExecution,
  InputTask,
  InputVectorCompletionTask,
  InputFunctionExecutionTask,
  InputVote,
  InputCompletion,
} from "./types";

export { DEFAULT_CONFIG, NODE_SIZES, SCORE_COLORS, scoreColor } from "./types";
