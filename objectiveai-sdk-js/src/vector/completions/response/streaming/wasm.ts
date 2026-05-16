import { vectorCompletionChunkMerged, vectorCompletionChunkNormalized, vectorCompletionChunkToUnary, generateVectorCompletionChunk, normalizeVectorCompletionForTests } from "../../../../wasm/loader.js";
import type { VectorCompletionsResponseStreamingVectorCompletionChunk } from "./vectorCompletionChunk";
import type { VectorCompletionsResponseUnaryVectorCompletion } from "../unary/vectorCompletion";

export function wasmVectorCompletionsResponseStreamingVectorCompletionChunkMerged(a: VectorCompletionsResponseStreamingVectorCompletionChunk, b: VectorCompletionsResponseStreamingVectorCompletionChunk): VectorCompletionsResponseStreamingVectorCompletionChunk {
  return JSON.parse(vectorCompletionChunkMerged(a, b));
}

export function wasmVectorCompletionsResponseStreamingVectorCompletionChunkNormalized(a: VectorCompletionsResponseStreamingVectorCompletionChunk): VectorCompletionsResponseStreamingVectorCompletionChunk {
  return JSON.parse(vectorCompletionChunkNormalized(a));
}

export function wasmVectorCompletionsResponseStreamingVectorCompletionChunkToUnary(a: VectorCompletionsResponseStreamingVectorCompletionChunk): VectorCompletionsResponseUnaryVectorCompletion {
  return JSON.parse(vectorCompletionChunkToUnary(a));
}

export function wasmVectorCompletionsResponseStreamingGenerateVectorCompletionChunk(seed: number): VectorCompletionsResponseStreamingVectorCompletionChunk {
  return JSON.parse(generateVectorCompletionChunk(seed));
}

export function wasmVectorCompletionsResponseStreamingNormalizeVectorCompletionForTests(a: VectorCompletionsResponseUnaryVectorCompletion): VectorCompletionsResponseUnaryVectorCompletion {
  return JSON.parse(normalizeVectorCompletionForTests(a));
}
