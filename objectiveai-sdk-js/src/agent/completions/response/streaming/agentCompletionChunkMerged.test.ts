import { describe, it, expect } from "vitest";
import { agentCompletionsResponseStreamingAgentCompletionChunkMerged } from "./agentCompletionChunkMerged";
import { AgentCompletionsResponseStreamingAgentCompletionChunkSchema } from "./agentCompletionChunk";
import {
  wasmAgentCompletionsResponseStreamingAgentCompletionChunkMerged as wasmMerged,
  wasmAgentCompletionsResponseStreamingGenerateAgentCompletionChunk as generate,
} from "./wasm";
import { rounded } from "../../../../mergeTestUtil";

describe("agentCompletionChunkMerged fuzz", () => {
  for (let i = 0; i < 20; i++) {
    it(`stream ${i}`, () => {
      let seed = i * 1000;
      let tsAcc = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.parse(generate(seed++));
      let wasmAcc = structuredClone(tsAcc);
      for (let j = 0; j < 20; j++) {
        const chunk = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.parse(generate(seed++));
        [tsAcc] = agentCompletionsResponseStreamingAgentCompletionChunkMerged(tsAcc, chunk);
        wasmAcc = wasmMerged(wasmAcc, chunk);
        expect(rounded(tsAcc), `chunk ${j}`).toEqual(rounded(wasmAcc));
      }
    });
  }
});
