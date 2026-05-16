import * as path from "path";
import { httpTestSuite } from "../../httpTestUtil";
import { functionsInventionsCreateFunctionInvention } from "./http";
import { functionsInventionsResponseStreamingFunctionInventionChunkMerged } from "./response/streaming/functionInventionChunkMerged";
import {
  wasmFunctionsInventionsResponseStreamingFunctionInventionChunkToUnary,
  wasmFunctionsInventionsResponseStreamingNormalizeFunctionInventionForTests as normalize,
} from "./response/streaming/wasm";
import type { FunctionsInventionsResponseStreamingFunctionInventionChunk } from "./response/streaming/functionInventionChunk";
import type { FunctionsInventionsResponseUnaryFunctionInvention } from "./response/unary/functionInvention";

const mockInventionAgent = { upstream: "mock", output_mode: "instruction", mode: "invention" };
const mockPrompt = { remote: "mock", name: "default" };

httpTestSuite<FunctionsInventionsResponseStreamingFunctionInventionChunk, FunctionsInventionsResponseUnaryFunctionInvention>({
  name: "functions inventions http",
  fn: functionsInventionsCreateFunctionInvention,
  snapshotsDir: path.resolve(__dirname, "../../../../objectiveai-api/assets/functions/inventions/client_tests"),
  merge: functionsInventionsResponseStreamingFunctionInventionChunkMerged,
  chunkToUnary: wasmFunctionsInventionsResponseStreamingFunctionInventionChunkToUnary,
  normalize,
  cases: [
    {
      snapshot: "scalar_leaf_s42_0",
      body: {
        state: {
          type: "alpha.scalar.leaf.function",
          depth: 0, min_branch_width: 3, max_branch_width: 5, min_leaf_width: 3, max_leaf_width: 5,
          name: "sl-default", spec: "Test function spec for mock invention.",
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 42,
        stream: true,
        max_step_retries: 1,
      },
    },
    {
      snapshot: "vector_branch_s2025_0",
      body: {
        state: {
          type: "alpha.vector.branch.function",
          depth: 3, min_branch_width: 2, max_branch_width: 4, min_leaf_width: 2, max_leaf_width: 4,
          name: "vb-deep", spec: "Test function spec for mock invention.",
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 2025,
        stream: true,
        max_step_retries: 1,
      },
    },
    {
      snapshot: "scalar_leaf_schema_kitchen_0",
      body: {
        state: {
          type: "alpha.scalar.leaf.function",
          depth: 0, min_branch_width: 3, max_branch_width: 5, min_leaf_width: 3, max_leaf_width: 5,
          name: "sl-kitchen", spec: "Test function spec for mock invention.",
          input_schema: {
            type: "object",
            properties: {
              name: { type: "string" },
              age: { type: "integer" },
              score: { type: "number" },
              active: { type: "boolean" },
              avatar: { type: "image" },
              voicemail: { type: "audio" },
              demo: { type: "video" },
              resume: { type: "file" },
              aliases: {
                type: "array",
                items: { anyOf: [{ type: "string" }, { type: "integer" }] },
                minItems: 1,
                maxItems: 8,
              },
              extra: {
                anyOf: [
                  { type: "string" },
                  {
                    type: "array",
                    items: {
                      type: "object",
                      properties: {
                        key: { type: "string" },
                        val: { anyOf: [{ type: "number" }, { type: "boolean" }, { type: "image" }] },
                      },
                      required: ["key", "val"],
                    },
                    minItems: 1,
                    maxItems: 3,
                  },
                ],
              },
            },
            required: ["name", "age", "score", "active", "avatar", "voicemail", "demo", "resume", "aliases", "extra"],
          },
        },
        agent: mockInventionAgent,
        prompt: mockPrompt,
        seed: 80004,
        stream: true,
        max_step_retries: 1,
      },
    },
  ],
});
