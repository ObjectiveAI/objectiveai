import * as path from "path";
import { httpTestSuite } from "../../httpTestUtil";
import {
  functionsExecutionsCreateFunctionExecution,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkToUnary,
  wasmFunctionsExecutionsResponseStreamingNormalizeFunctionExecutionForTests as normalize,
  type FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  type FunctionsExecutionsResponseUnaryFunctionExecution,
} from "@objectiveai/sdk";

httpTestSuite<FunctionsExecutionsResponseStreamingFunctionExecutionChunk, FunctionsExecutionsResponseUnaryFunctionExecution>({
  name: "functions executions http",
  fn: functionsExecutionsCreateFunctionExecution,
  snapshotsDir: path.resolve(__dirname, "../../../../objectiveai-api-tests/assets/functions/executions/client_tests"),
  merge: functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  chunkToUnary: wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkToUnary,
  normalize,
  cases: [
    {
      snapshot: "mock_1_scalar_leaf_binary_seed_42",
      body: { function: { remote: "mock", name: "binary-classifier" }, profile: { remote: "mock", name: "solo-instruction" }, input: { text: "Hello world" }, seed: 42 },
    },
    {
      snapshot: "mock_7_vector_5_criteria_seed_42",
      body: { function: { remote: "mock", name: "five-criteria-ranker" }, profile: { remote: "mock", name: "schema-heavy-trio" }, input: { items: ["Option A", "Option B", "Option C"] }, seed: 42 },
    },
    {
      snapshot: "mock_20_vector_super_branch_seed_42",
      body: { function: { remote: "mock", name: "nested-vector-super-branch" }, profile: { remote: "mock", name: "nested-vector-inline-remote" }, input: { items: ["Alpha", "Beta", "Gamma"] }, seed: 42 },
    },
  ],
});
