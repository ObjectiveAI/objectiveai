// ---------------------------------------------------------------------------
// @objectiveai/function-tree — Public API
// ---------------------------------------------------------------------------

// React component
export { FunctionTree } from "./react/FunctionTree";

// Core engine (for framework-agnostic usage)
export { FunctionTreeEngine } from "./core/engine";

// Data transformation
export { buildTree, applyProfileWeights } from "./core/tree-data";
export { buildStructuralTree, extractPromptPreview } from "./core/structural-tree-data";

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
  TreeMode,
  FunctionNodeData,
  VectorCompletionNodeData,
  EnsembleLlmNodeData,
  FunctionTreeConfig,
  FunctionTreeProps,
  InputFunctionExecution,
  InputFunctionDefinition,
  InputTaskDefinition,
  InputTask,
  InputVectorCompletionTask,
  InputFunctionExecutionTask,
  InputVote,
  InputCompletion,
  InputProfile,
  InputProfileTask,
  InputProfileEnsembleLlm,
} from "./types";

export { DEFAULT_CONFIG, NODE_SIZES, SCORE_COLORS, scoreColor } from "./types";
