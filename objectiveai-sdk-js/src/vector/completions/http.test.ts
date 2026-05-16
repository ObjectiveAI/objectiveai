import * as path from "path";
import { httpTestSuite } from "../../httpTestUtil";
import { vectorCompletionsCreateVectorCompletion } from "./http";
import { vectorCompletionsResponseStreamingVectorCompletionChunkMerged } from "./response/streaming/vectorCompletionChunkMerged";
import {
  wasmVectorCompletionsResponseStreamingVectorCompletionChunkToUnary,
  wasmVectorCompletionsResponseStreamingNormalizeVectorCompletionForTests as normalize,
} from "./response/streaming/wasm";
import type { VectorCompletionsResponseStreamingVectorCompletionChunk } from "./response/streaming/vectorCompletionChunk";
import type { VectorCompletionsResponseUnaryVectorCompletion } from "./response/unary/vectorCompletion";

const mockAgent = { upstream: "mock", output_mode: "instruction" };

httpTestSuite<VectorCompletionsResponseStreamingVectorCompletionChunk, VectorCompletionsResponseUnaryVectorCompletion>({
  name: "vector completions http",
  fn: vectorCompletionsCreateVectorCompletion,
  snapshotsDir: path.resolve(__dirname, "../../../../objectiveai-api/assets/vector/completions/client_tests"),
  merge: vectorCompletionsResponseStreamingVectorCompletionChunkMerged,
  chunkToUnary: wasmVectorCompletionsResponseStreamingVectorCompletionChunkToUnary,
  normalize,
  cases: [
    {
      snapshot: "single_agent_2_responses_instruction_seed_42",
      body: {
        messages: [{ role: "user", content: "Which is better?" }],
        swarm: { agents: [{ ...mockAgent }], weights: [1] },
        responses: ["Response A", "Response B"],
        seed: 42,
      },
    },
    {
      snapshot: "many_responses_deep_prefix_tree_seed_42",
      body: {
        messages: [{ role: "user", content: "Pick the best" }],
        swarm: { agents: [{ ...mockAgent }], weights: [1] },
        responses: Array.from({ length: 25 }, (_, i) => `Response ${i}`),
        seed: 42,
      },
    },
    {
      snapshot: "mixed_output_modes_seed_88",
      body: {
        messages: [{ role: "user", content: "Compare these vacation destinations" }],
        swarm: {
          agents: [
            { upstream: "mock", output_mode: "instruction" },
            { upstream: "mock", output_mode: "json_schema" },
            { upstream: "mock", output_mode: "tool_call" },
          ],
          weights: [0.4, 0.3, 0.3],
        },
        responses: ["Kyoto, Japan", "Reykjavik, Iceland", "Patagonia, Argentina"],
        seed: 88,
      },
    },
  ],
});
