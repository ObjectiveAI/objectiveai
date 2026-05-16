import { describe, it, expect } from "vitest";
import { vectorCompletionsResponseStreamingVectorCompletionChunkMerged } from "./vectorCompletionChunkMerged";
import { VectorCompletionsResponseStreamingVectorCompletionChunkSchema } from "./vectorCompletionChunk";
import {
  wasmVectorCompletionsResponseStreamingVectorCompletionChunkMerged as wasmMerged,
  wasmVectorCompletionsResponseStreamingGenerateVectorCompletionChunk as generate,
} from "./wasm";
import { rounded } from "../../../../mergeTestUtil";

describe("vectorCompletionChunkMerged fuzz", () => {
  for (let i = 0; i < 20; i++) {
    it(`stream ${i}`, () => {
      let seed = i * 1000;
      let tsAcc = VectorCompletionsResponseStreamingVectorCompletionChunkSchema.parse(generate(seed++));
      let wasmAcc = structuredClone(tsAcc);
      for (let j = 0; j < 20; j++) {
        const chunk = VectorCompletionsResponseStreamingVectorCompletionChunkSchema.parse(generate(seed++));
        [tsAcc] = vectorCompletionsResponseStreamingVectorCompletionChunkMerged(tsAcc, chunk);
        wasmAcc = wasmMerged(wasmAcc, chunk);
        expect(rounded(tsAcc), `chunk ${j}`).toEqual(rounded(wasmAcc));
      }
    });
  }
});
