import * as path from "path";
import { httpTestSuite } from "../../../httpTestUtil";
import { functionsInventionsRecursiveCreateFunctionInventionRecursive } from "./http";
import { functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged } from "./response/streaming/functionInventionRecursiveChunkMerged";
import {
  wasmFunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkToUnary,
  wasmFunctionsInventionsRecursiveResponseStreamingNormalizeFunctionInventionRecursiveForTests as normalize,
} from "./response/streaming/wasm";
import type { FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk } from "./response/streaming/functionInventionRecursiveChunk";
import type { FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive } from "./response/unary/functionInventionRecursive";

const mockInventionAgent = { upstream: "mock", output_mode: "instruction", mode: "invention" };
const mockPrompt = { remote: "mock", name: "default" };

httpTestSuite<FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk, FunctionsInventionsRecursiveResponseUnaryFunctionInventionRecursive>({
  name: "functions inventions recursive http",
  fn: functionsInventionsRecursiveCreateFunctionInventionRecursive,
  snapshotsDir: path.resolve(__dirname, "../../../../../objectiveai-api/assets/functions/inventions/recursive_client_tests"),
  merge: functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  chunkToUnary: wasmFunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkToUnary,
  normalize,
  cases: [
    {
      snapshot: "valid_schema_valid_tasks_scalar_leaf",
      body: {
        remote: "mock",
        state: {
          type: "alpha.scalar.leaf.function",
          depth: 0, min_branch_width: 1, max_branch_width: 1, min_leaf_width: 2, max_leaf_width: 4,
          name: "inv-good-sl", spec: "Test function spec for mock invention.",
          input_schema: {
            type: "object",
            properties: {
              sentiment: { type: "string", enum: ["positive", "negative"] },
            },
            required: ["sentiment"],
          },
          essay_tasks: "Good tasks incoming.",
          tasks: [
            {
              type: "vector.completion",
              messages: { $starlark: "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": str(input)}]}]" },
              responses: [[{ type: "text", text: "yes" }], [{ type: "text", text: "no" }]],
            },
            {
              type: "vector.completion",
              messages: { $starlark: "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": str(input)}]}]" },
              responses: [[{ type: "text", text: "yes" }], [{ type: "text", text: "no" }]],
            },
          ],
          tasks_length: 2,
          description: "A valid scalar function.",
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 5300,
        stream: true,
        max_step_retries: 1,
      },
    },
    {
      snapshot: "valid_vector_schema_valid_tasks",
      body: {
        remote: "mock",
        state: {
          type: "alpha.vector.leaf.function",
          depth: 0, min_branch_width: 1, max_branch_width: 1, min_leaf_width: 2, max_leaf_width: 4,
          name: "inv-good-vl", spec: "Test function spec for mock invention.",
          essay: "Ranking things.",
          input_schema: {
            items: { type: "string", enum: ["apple", "banana"] },
          },
          tasks: [
            {
              type: "vector.completion",
              messages: { $starlark: "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"rank these\"}]}]" },
              responses: { $starlark: "[[{\"type\": \"text\", \"text\": str(item)}] for item in input['items']]" },
            },
            {
              type: "vector.completion",
              messages: { $starlark: "[{\"role\": \"user\", \"content\": [{\"type\": \"text\", \"text\": \"rank these\"}]}]" },
              responses: { $starlark: "[[{\"type\": \"text\", \"text\": str(item)}] for item in input['items']]" },
            },
          ],
          tasks_length: 2,
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 5400,
        stream: true,
        max_step_retries: 1,
      },
    },
    {
      snapshot: "valid_schema_no_tasks_with_essay",
      body: {
        remote: "mock",
        state: {
          type: "alpha.scalar.leaf.function",
          depth: 0, min_branch_width: 1, max_branch_width: 1, min_leaf_width: 2, max_leaf_width: 4,
          name: "inv-schema-only", spec: "Test function spec for mock invention.",
          essay: "A great essay about things.",
          input_schema: {
            type: "object",
            properties: {
              sentiment: { type: "string", enum: ["positive", "negative"] },
            },
            required: ["sentiment"],
          },
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 5900,
        stream: true,
        max_step_retries: 1,
      },
    },
  ],
});
