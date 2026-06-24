import * as path from "path";
import { httpTestSuite } from "../../httpTestUtil";
import {
  agentCompletionsCreateAgentCompletion,
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  wasmAgentCompletionsResponseStreamingAgentCompletionChunkToUnary,
  wasmAgentCompletionsResponseStreamingNormalizeAgentCompletionForTests as normalize,
  type AgentCompletionsResponseStreamingAgentCompletionChunk,
  type AgentCompletionsResponseUnaryAgentCompletion,
} from "@objectiveai/sdk";

httpTestSuite<AgentCompletionsResponseStreamingAgentCompletionChunk, AgentCompletionsResponseUnaryAgentCompletion>({
  name: "agent completions http",
  fn: agentCompletionsCreateAgentCompletion,
  snapshotsDir: path.resolve(__dirname, "../../../../objectiveai-api-tests/assets/agent/completions/client_tests"),
  merge: agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  chunkToUnary: wasmAgentCompletionsResponseStreamingAgentCompletionChunkToUnary,
  normalize,
  cases: [
    { snapshot: "test_basic_mock_agent_seed_42", body: { messages: [], agent: { upstream: "mock", output_mode: "instruction" }, seed: 42 } },
    {
      snapshot: "test_with_developer_and_user_messages",
      body: {
        messages: [
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
