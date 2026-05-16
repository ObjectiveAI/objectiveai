import { laboratoryExecutionChunkMerged, laboratoryExecutionChunkNormalized, laboratoryExecutionChunkToUnary, generateLaboratoryExecutionChunk, normalizeLaboratoryExecutionForTests } from "../../../../wasm/loader.js";
import type { LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk } from "./laboratoryExecutionChunk";
import type { LaboratoriesExecutionsResponseUnaryLaboratoryExecution } from "../unary/laboratoryExecution";

export function wasmLaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(a: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk, b: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk): LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk {
  return JSON.parse(laboratoryExecutionChunkMerged(a, b));
}

export function wasmLaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkNormalized(a: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk): LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk {
  return JSON.parse(laboratoryExecutionChunkNormalized(a));
}

export function wasmLaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkToUnary(a: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk): LaboratoriesExecutionsResponseUnaryLaboratoryExecution {
  return JSON.parse(laboratoryExecutionChunkToUnary(a));
}

export function wasmLaboratoriesExecutionsResponseStreamingGenerateLaboratoryExecutionChunk(seed: number): LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk {
  return JSON.parse(generateLaboratoryExecutionChunk(seed));
}

export function wasmLaboratoriesExecutionsResponseStreamingNormalizeLaboratoryExecutionForTests(a: LaboratoriesExecutionsResponseUnaryLaboratoryExecution): LaboratoriesExecutionsResponseUnaryLaboratoryExecution {
  return JSON.parse(normalizeLaboratoryExecutionForTests(a));
}
