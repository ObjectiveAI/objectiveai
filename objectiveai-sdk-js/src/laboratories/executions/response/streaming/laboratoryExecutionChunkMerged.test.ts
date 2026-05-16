import { describe, it, expect } from "vitest";
import { laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged } from "./laboratoryExecutionChunkMerged";
import { LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema } from "./laboratoryExecutionChunk";
import {
  wasmLaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged as wasmMerged,
  wasmLaboratoriesExecutionsResponseStreamingGenerateLaboratoryExecutionChunk as generate,
} from "./wasm";
import { rounded } from "../../../../mergeTestUtil";

describe("laboratoryExecutionChunkMerged fuzz", () => {
  for (let i = 0; i < 20; i++) {
    it(`stream ${i}`, () => {
      let seed = i * 1000;
      let tsAcc = LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema.parse(generate(seed++));
      let wasmAcc = structuredClone(tsAcc);
      for (let j = 0; j < 20; j++) {
        const chunk = LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema.parse(generate(seed++));
        [tsAcc] = laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(tsAcc, chunk);
        wasmAcc = wasmMerged(wasmAcc, chunk);
        expect(rounded(tsAcc), `chunk ${j}`).toEqual(rounded(wasmAcc));
      }
    });
  }
});
