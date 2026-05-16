import * as path from "path";
import { httpTestSuite } from "../../httpTestUtil";
import { agentCompletionsCreateAgentCompletion } from "./http";
import { agentCompletionsResponseStreamingAgentCompletionChunkMerged } from "./response/streaming/agentCompletionChunkMerged";
import {
  wasmAgentCompletionsResponseStreamingAgentCompletionChunkToUnary,
  wasmAgentCompletionsResponseStreamingNormalizeAgentCompletionForTests as normalize,
} from "./response/streaming/wasm";
import type { AgentCompletionsResponseStreamingAgentCompletionChunk } from "./response/streaming/agentCompletionChunk";
import type { AgentCompletionsResponseUnaryAgentCompletion } from "./response/unary/agentCompletion";

httpTestSuite<AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseUnaryAgentCompletion>({
  name: "agent completions http",
  fn: agentCompletionsCreateAgentCompletion,
  snapshotsDir: path.resolve(__dirname, "../../../../objectiveai-api/assets/agent/completions/client_tests"),
  merge: agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  chunkToUnary: wasmAgentCompletionsResponseStreamingAgentCompletionChunkToUnary,
  normalize,
  cases: [
    { snapshot: "test_basic_mock_agent_seed_42", body: { messages: [], agent: { upstream: "mock", output_mode: "instruction" }, seed: 42 } },
    {
      snapshot: "test_with_developer_and_user_messages",
      body: {
        messages: [
          { role: "developer", content: "You are a helpful assistant." },
          { role: "user", content: "What is 2+2?" },
        ],
        agent: { upstream: "mock", output_mode: "instruction" },
        seed: 99,
      },
    },
    {
      snapshot: "test_json_object_response_format",
      body: { messages: [], agent: { upstream: "mock", output_mode: "instruction" }, response_format: { type: "json_object" }, seed: 42 },
    },
  ],
});
