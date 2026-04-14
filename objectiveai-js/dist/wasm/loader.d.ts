/* tslint:disable */
/* eslint-disable */

/**
 * Merges two `AgentCompletionChunk`s and returns the merged result.
 */
export function agentCompletionChunkMerged(a: any, b: any): string;

/**
 * Normalizes an `AgentCompletionChunk` by round-tripping through serde.
 */
export function agentCompletionChunkNormalized(a: any): string;

/**
 * Converts an accumulated `AgentCompletionChunk` to an `AgentCompletion` (unary).
 */
export function agentCompletionChunkToUnary(a: any): string;

/**
 * Alpha check for a branch scalar function (depth > 0, scalar output).
 *
 * `children` is an optional map of child function name → FullRemoteFunction for
 * validating placeholder task inputs against child function input schemas.
 */
export function alphaCheckBranchScalarFunction(_function: any, children: any): void;

/**
 * Alpha check for a branch vector function (depth > 0, vector output).
 *
 * `children` is an optional map of child function name → FullRemoteFunction for
 * validating placeholder task inputs against child function input schemas.
 */
export function alphaCheckBranchVectorFunction(_function: any, children: any): void;

/**
 * Alpha check for a leaf scalar function (depth 0, scalar output).
 */
export function alphaCheckLeafScalarFunction(_function: any): void;

/**
 * Alpha check for a leaf vector function (depth 0, vector output).
 */
export function alphaCheckLeafVectorFunction(_function: any): void;

/**
 * Validates scalar function fields (input_schema only).
 */
export function checkScalarFields(fields: any): void;

/**
 * Validates vector function fields (output_length, input_split, input_merge).
 *
 * Generates diverse example inputs from the input_schema and validates that the
 * output_length, input_split, and input_merge expressions work correctly together
 * via round-trip testing.
 */
export function checkVectorFields(fields: any): void;

/**
 * Compiles the `input_merge` expression to merge multiple sub-inputs back into one.
 *
 * Used by strategies like Swiss System to recombine a subset of split inputs
 * into a single input for pool execution. The expression transforms an array
 * of inputs (a subset from `compileFunctionInputSplit`) into a single merged input.
 *
 * # Arguments
 *
 * * `function` - JavaScript object representing a Function definition
 * * `input` - Array of inputs to merge (typically a subset from `compileFunctionInputSplit`)
 *
 * # Returns
 *
 * - The merged input for vector functions with `input_merge` defined
 * - `null` for scalar functions or functions without `input_merge`
 *
 * # Errors
 *
 * Returns an error string if expression evaluation fails.
 */
export function compileFunctionInputMerge(_function: any, input: any): string | undefined;

/**
 * Compiles the `input_split` expression to split input into multiple sub-inputs.
 *
 * Used by strategies like Swiss System that need to partition input into
 * smaller pools. The expression transforms the original input into an array
 * of inputs, where each element can be processed independently.
 *
 * # Arguments
 *
 * * `function` - JavaScript object representing a Function definition
 * * `input` - JavaScript object representing the function input to split
 *
 * # Returns
 *
 * - An array of split inputs for vector functions with `input_split` defined
 * - `null` for scalar functions or functions without `input_split`
 *
 * # Errors
 *
 * Returns an error string if expression evaluation fails.
 */
export function compileFunctionInputSplit(_function: any, input: any): string | undefined;

/**
 * Computes the expected output length for a vector Function.
 *
 * Evaluates the `output_length` expression to determine how many elements
 * the output vector should contain. This is only applicable to remote
 * vector functions which have an `output_length` field.
 *
 * # Arguments
 *
 * * `function` - JavaScript object representing a Function definition
 * * `input` - JavaScript object representing the function input
 *
 * # Returns
 *
 * - The expected output length for remote vector functions
 * - `null` for scalar functions or inline functions
 *
 * # Errors
 *
 * Returns an error string if expression evaluation fails.
 */
export function compileFunctionOutputLength(_function: any, input: any): number | undefined;

/**
 * Compiles a Function's task expressions for a given input.
 *
 * Evaluates all expressions (JMESPath or Starlark) in the function's tasks
 * using the provided input data. This is used for previewing how tasks will
 * be executed during Function authoring.
 *
 * # Arguments
 *
 * * `function` - JavaScript object representing a Function definition
 * * `input` - JavaScript object representing the function input
 *
 * # Returns
 *
 * An array where each element corresponds to a task definition:
 * - `null` if the task was skipped (skip expression evaluated to true)
 * - `{ One: task }` for non-mapped tasks
 * - `{ Many: [task, ...] }` for mapped tasks (expanded from map expression)
 *
 * # Errors
 *
 * Returns an error string if expression evaluation fails or types don't match.
 */
export function compileFunctionTasks(_function: any, input: any): string;

/**
 * Merges two `FunctionExecutionChunk`s and returns the merged result.
 */
export function functionExecutionChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `FunctionExecutionChunk` by round-tripping through serde.
 */
export function functionExecutionChunkNormalized(a: any): string;

/**
 * Converts an accumulated `FunctionExecutionChunk` to a `FunctionExecution` (unary).
 */
export function functionExecutionChunkToUnary(a: any): string;

/**
 * Merges two `FunctionInventionChunk`s and returns the merged result.
 */
export function functionInventionChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `FunctionInventionChunk` by round-tripping through serde.
 */
export function functionInventionChunkNormalized(a: any): string;

/**
 * Converts an accumulated `FunctionInventionChunk` to a `FunctionInvention` (unary).
 */
export function functionInventionChunkToUnary(a: any): string;

/**
 * Merges two `FunctionInventionRecursiveChunk`s and returns the merged result.
 */
export function functionInventionRecursiveChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `FunctionInventionRecursiveChunk` by round-tripping through serde.
 */
export function functionInventionRecursiveChunkNormalized(a: any): string;

/**
 * Converts an accumulated `FunctionInventionRecursiveChunk` to a `FunctionInventionRecursive` (unary).
 */
export function functionInventionRecursiveChunkToUnary(a: any): string;

/**
 * Merges two `FunctionProfileComputationChunk`s and returns the merged result.
 */
export function functionProfileComputationChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `FunctionProfileComputationChunk` by round-tripping through serde.
 */
export function functionProfileComputationChunkNormalized(a: any): string;

/**
 * Converts an accumulated `FunctionProfileComputationChunk` to a `FunctionProfileComputation` (unary).
 */
export function functionProfileComputationChunkToUnary(a: any): string;

/**
 * Generates a random `AgentCompletionChunk`. Optional seed for reproducibility.
 */
export function generateAgentCompletionChunk(seed: any): string;

/**
 * Generates a random `FunctionExecutionChunk`. Optional seed for reproducibility.
 */
export function generateFunctionExecutionChunk(seed: any): string;

/**
 * Generates a random `FunctionInventionChunk`. Optional seed for reproducibility.
 */
export function generateFunctionInventionChunk(seed: any): string;

/**
 * Generates a random `FunctionInventionRecursiveChunk`. Optional seed for reproducibility.
 */
export function generateFunctionInventionRecursiveChunk(seed: any): string;

/**
 * Generates a random `FunctionProfileComputationChunk`. Optional seed for reproducibility.
 */
export function generateFunctionProfileComputationChunk(seed: any): string;

/**
 * Generates a random `LaboratoryExecutionChunk`. Optional seed for reproducibility.
 */
export function generateLaboratoryExecutionChunk(seed: any): string;

/**
 * Generates a random `VectorCompletionChunk`. Optional seed for reproducibility.
 */
export function generateVectorCompletionChunk(seed: any): string;

/**
 * Merges two `LaboratoryExecutionChunk`s and returns the merged result.
 */
export function laboratoryExecutionChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `LaboratoryExecutionChunk` by round-tripping through serde.
 */
export function laboratoryExecutionChunkNormalized(a: any): string;

/**
 * Converts an accumulated `LaboratoryExecutionChunk` to a `LaboratoryExecution` (unary).
 */
export function laboratoryExecutionChunkToUnary(a: any): string;

/**
 * Normalizes an `AgentCompletion` for test snapshot stability.
 */
export function normalizeAgentCompletionForTests(a: any): string;

/**
 * Normalizes a `FunctionExecution` for test snapshot stability.
 */
export function normalizeFunctionExecutionForTests(a: any): string;

/**
 * Normalizes a `FunctionInvention` for test snapshot stability.
 */
export function normalizeFunctionInventionForTests(a: any): string;

/**
 * Normalizes a `FunctionInventionRecursive` for test snapshot stability.
 */
export function normalizeFunctionInventionRecursiveForTests(a: any): string;

/**
 * Normalizes a `FunctionProfileComputation` for test snapshot stability.
 */
export function normalizeFunctionProfileComputationForTests(a: any): string;

/**
 * Normalizes a `LaboratoryExecution` for test snapshot stability.
 */
export function normalizeLaboratoryExecutionForTests(a: any): string;

/**
 * Normalizes a `VectorCompletion` for test snapshot stability.
 */
export function normalizeVectorCompletionForTests(a: any): string;

/**
 * Computes a content-addressed ID for chat messages.
 *
 * Normalizes the messages (consolidates text parts, removes empty content)
 * and computes a deterministic hash. This ID is used for caching and
 * deduplicating requests with identical prompts.
 *
 * # Arguments
 *
 * * `prompt` - Array of chat messages
 *
 * # Returns
 *
 * A base62-encoded hash string uniquely identifying the prompt content.
 *
 * # Errors
 *
 * Returns an error if the messages cannot be deserialized.
 */
export function promptId(prompt: any): string;

/**
 * Validates an Agent configuration and computes its content-addressed ID.
 *
 * Takes an Agent definition, normalizes it (removes defaults, deduplicates),
 * validates all fields, and computes a deterministic ID using XXHash3-128.
 *
 * # Arguments
 *
 * * `agent` - JavaScript object representing an Agent configuration
 *
 * # Returns
 *
 * The validated Agent with its computed `id` field populated.
 *
 * # Errors
 *
 * Returns an error string if validation fails (e.g., invalid model name,
 * out-of-range parameters, conflicting settings).
 */
export function validateAgent(agent: any): string;

/**
 * Validates function input against its schema.
 *
 * For remote functions, checks whether the provided input conforms to
 * the function's JSON Schema definition. For inline functions, returns
 * `null` since they lack schema definitions.
 *
 * # Arguments
 *
 * * `function` - JavaScript object representing a Function definition
 * * `input` - JavaScript object representing the function input to validate
 *
 * # Returns
 *
 * - `true` if the input is valid against the schema
 * - `false` if the input is invalid
 * - `null` for inline functions (no schema to validate against)
 *
 * # Errors
 *
 * Returns an error if deserialization fails.
 */
export function validateFunctionInput(_function: any, input: any): boolean | undefined;

/**
 * Validates an Swarm configuration and computes its content-addressed ID.
 *
 * Takes an Swarm definition (a collection of Swarm LLMs), validates each
 * LLM, and computes a deterministic ID for the swarm as a whole.
 *
 * # Arguments
 *
 * * `swarm` - JavaScript object representing an Swarm configuration
 *
 * # Returns
 *
 * The validated Swarm with its computed `id` field populated and all
 * member LLMs validated with their IDs.
 *
 * # Errors
 *
 * Returns an error string if any LLM validation fails or the swarm
 * structure is invalid.
 */
export function validateSwarm(swarm: any, remote_agents: any): string;

/**
 * Merges two `VectorCompletionChunk`s and returns the merged result.
 */
export function vectorCompletionChunkMerged(a: any, b: any): string;

/**
 * Normalizes a `VectorCompletionChunk` by round-tripping through serde.
 */
export function vectorCompletionChunkNormalized(a: any): string;

/**
 * Converts an accumulated `VectorCompletionChunk` to a `VectorCompletion` (unary).
 */
export function vectorCompletionChunkToUnary(a: any): string;

/**
 * Computes a content-addressed ID for a vector completion response option.
 *
 * Normalizes the response content (consolidates text parts, removes empty
 * content) and computes a deterministic hash. This ID is used for caching
 * and identifying individual response options in vector completions.
 *
 * # Arguments
 *
 * * `response` - A rich content object (text or multipart content)
 *
 * # Returns
 *
 * A base62-encoded hash string uniquely identifying the response content.
 *
 * # Errors
 *
 * Returns an error if the response cannot be deserialized.
 */
export function vectorResponseId(response: any): string;
