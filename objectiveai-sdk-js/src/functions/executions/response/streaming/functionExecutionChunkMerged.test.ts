import { describe, it, expect } from "vitest";
import { functionsExecutionsResponseStreamingFunctionExecutionChunkMerged } from "./functionExecutionChunkMerged";
import { FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema } from "./functionExecutionChunk";
import {
  wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkMerged as wasmMerged,
  wasmFunctionsExecutionsResponseStreamingGenerateFunctionExecutionChunk as generate,
} from "./wasm";
import { rounded } from "../../../../mergeTestUtil";

describe("functionExecutionChunkMerged fuzz", () => {
  for (let i = 0; i < 20; i++) {
    it(`stream ${i}`, () => {
      let seed = i * 1000;
      let tsAcc = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.parse(generate(seed++));
      let wasmAcc = structuredClone(tsAcc);
      for (let j = 0; j < 20; j++) {
        const chunk = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.parse(generate(seed++));
        [tsAcc] = functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(tsAcc, chunk);
        wasmAcc = wasmMerged(wasmAcc, chunk);
        expect(rounded(tsAcc), `chunk ${j}`).toEqual(rounded(wasmAcc));
      }
    });
  }
});
