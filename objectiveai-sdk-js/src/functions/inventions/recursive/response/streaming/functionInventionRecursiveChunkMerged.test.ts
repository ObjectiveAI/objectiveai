import { describe, it, expect } from "vitest";
import { functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged } from "./functionInventionRecursiveChunkMerged";
import { FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema } from "./functionInventionRecursiveChunk";
import {
  wasmFunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged as wasmMerged,
  wasmFunctionsInventionsRecursiveResponseStreamingGenerateFunctionInventionRecursiveChunk as generate,
} from "./wasm";
import { rounded } from "../../../../../mergeTestUtil";

describe("functionInventionRecursiveChunkMerged fuzz", () => {
  for (let i = 0; i < 20; i++) {
    it(`stream ${i}`, () => {
      let seed = i * 1000;
      let tsAcc = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.parse(generate(seed++));
      let wasmAcc = structuredClone(tsAcc);
      for (let j = 0; j < 20; j++) {
        const chunk = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.parse(generate(seed++));
        [tsAcc] = functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(tsAcc, chunk);
        wasmAcc = wasmMerged(wasmAcc, chunk);
        expect(rounded(tsAcc), `chunk ${j}`).toEqual(rounded(wasmAcc));
      }
    });
  }
});
