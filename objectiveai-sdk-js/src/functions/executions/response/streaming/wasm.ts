import { functionExecutionChunkMerged, functionExecutionChunkNormalized, functionExecutionChunkToUnary, generateFunctionExecutionChunk, normalizeFunctionExecutionForTests } from "../../../../wasm/loader.js";
import type { FunctionsExecutionsResponseStreamingFunctionExecutionChunk } from "./functionExecutionChunk";
import type { FunctionsExecutionsResponseUnaryFunctionExecution } from "../unary/functionExecution";

export function wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkMerged(a: FunctionsExecutionsResponseStreamingFunctionExecutionChunk, b: FunctionsExecutionsResponseStreamingFunctionExecutionChunk): FunctionsExecutionsResponseStreamingFunctionExecutionChunk {
  return JSON.parse(functionExecutionChunkMerged(a, b));
}

export function wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkNormalized(a: FunctionsExecutionsResponseStreamingFunctionExecutionChunk): FunctionsExecutionsResponseStreamingFunctionExecutionChunk {
  return JSON.parse(functionExecutionChunkNormalized(a));
}

export function wasmFunctionsExecutionsResponseStreamingFunctionExecutionChunkToUnary(a: FunctionsExecutionsResponseStreamingFunctionExecutionChunk): FunctionsExecutionsResponseUnaryFunctionExecution {
  return JSON.parse(functionExecutionChunkToUnary(a));
}

export function wasmFunctionsExecutionsResponseStreamingGenerateFunctionExecutionChunk(seed: number): FunctionsExecutionsResponseStreamingFunctionExecutionChunk {
  return JSON.parse(generateFunctionExecutionChunk(seed));
}

export function wasmFunctionsExecutionsResponseStreamingNormalizeFunctionExecutionForTests(a: FunctionsExecutionsResponseUnaryFunctionExecution): FunctionsExecutionsResponseUnaryFunctionExecution {
  return JSON.parse(normalizeFunctionExecutionForTests(a));
}
