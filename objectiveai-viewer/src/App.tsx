import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { AgentCompletionView } from "./AgentCompletionView";
import { FunctionInventionRecursiveView } from "./FunctionInventionRecursiveView";
import { FunctionExecutionView } from "./FunctionExecutionView";
import { replayMockEvents } from "./dev";
import { z } from "zod";
import {
  AgentCompletionsRequestAgentCompletionCreateParamsSchema,
  AgentCompletionsResponseStreamingAgentCompletionChunkSchema,
  FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema,
  FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema,
  LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema,
  ErrorResponseErrorSchema,
  agentCompletionsResponseStreamingAgentCompletionChunkMerged,
  functionsExecutionsResponseStreamingFunctionExecutionChunkMerged,
  functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged,
  laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged,
} from "objectiveai";
import type {
  AgentCompletionsResponseStreamingAgentCompletionChunk,
  FunctionsExecutionsResponseStreamingFunctionExecutionChunk,
  FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk,
  LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk,
} from "objectiveai";

// Extended schemas with required id
const AgentCompletionCreateParamsSchema = AgentCompletionsRequestAgentCompletionCreateParamsSchema.extend({
  id: z.string(),
});
type AgentCompletionCreateParams = z.infer<typeof AgentCompletionCreateParamsSchema>;

const FunctionExecutionCreateParamsSchema = FunctionsExecutionsRequestFunctionExecutionCreateParamsSchema.extend({
  id: z.string(),
});
type FunctionExecutionCreateParams = z.infer<typeof FunctionExecutionCreateParamsSchema>;

const FunctionInventionRecursiveCreateParamsSchema = FunctionsInventionsRecursiveRequestFunctionInventionRecursiveCreateParamsSchema.extend({
  id: z.string(),
});
type FunctionInventionRecursiveCreateParams = z.infer<typeof FunctionInventionRecursiveCreateParamsSchema>;

const LaboratoryExecutionCreateParamsSchema = LaboratoriesExecutionsRequestLaboratoryExecutionCreateParamsSchema.extend({
  id: z.string(),
});
type LaboratoryExecutionCreateParams = z.infer<typeof LaboratoryExecutionCreateParamsSchema>;

const ResponseErrorSchema = ErrorResponseErrorSchema.extend({
  id: z.string(),
});
type ResponseError = z.infer<typeof ResponseErrorSchema>;

// Classified incoming event
type AgentCompletionEvent =
  | { type: "begin"; data: AgentCompletionCreateParams }
  | { type: "chunk"; data: AgentCompletionsResponseStreamingAgentCompletionChunk }
  | { type: "error"; data: ResponseError };

type FunctionExecutionEvent =
  | { type: "begin"; data: FunctionExecutionCreateParams }
  | { type: "chunk"; data: FunctionsExecutionsResponseStreamingFunctionExecutionChunk }
  | { type: "error"; data: ResponseError };

type FunctionInventionRecursiveEvent =
  | { type: "begin"; data: FunctionInventionRecursiveCreateParams }
  | { type: "chunk"; data: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk }
  | { type: "error"; data: ResponseError };

type LaboratoryExecutionEvent =
  | { type: "begin"; data: LaboratoryExecutionCreateParams }
  | { type: "chunk"; data: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk }
  | { type: "error"; data: ResponseError };

// Entry in the list
interface AgentCompletionEntry {
  kind: "agent-completion";
  id: string;
  request: AgentCompletionCreateParams;
  chunk: AgentCompletionsResponseStreamingAgentCompletionChunk | null;
  error: ResponseError | null;
}

interface FunctionExecutionEntry {
  kind: "execution";
  id: string;
  request: FunctionExecutionCreateParams;
  chunk: FunctionsExecutionsResponseStreamingFunctionExecutionChunk | null;
  error: ResponseError | null;
}

interface FunctionInventionRecursiveEntry {
  kind: "invention";
  id: string;
  request: FunctionInventionRecursiveCreateParams;
  chunk: FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunk | null;
  error: ResponseError | null;
}

interface LaboratoryExecutionEntry {
  kind: "laboratory";
  id: string;
  request: LaboratoryExecutionCreateParams;
  chunk: LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunk | null;
  error: ResponseError | null;
}

type Entry = AgentCompletionEntry | FunctionExecutionEntry | FunctionInventionRecursiveEntry | LaboratoryExecutionEntry;

function classifyAgentCompletion(payload: unknown): AgentCompletionEvent | null {
  const beginParse = AgentCompletionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = AgentCompletionsResponseStreamingAgentCompletionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyFunctionExecution(payload: unknown): FunctionExecutionEvent | null {
  const beginParse = FunctionExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsExecutionsResponseStreamingFunctionExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyFunctionInventionRecursive(payload: unknown): FunctionInventionRecursiveEvent | null {
  const beginParse = FunctionInventionRecursiveCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = FunctionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function classifyLaboratoryExecution(payload: unknown): LaboratoryExecutionEvent | null {
  const beginParse = LaboratoryExecutionCreateParamsSchema.safeParse(payload);
  if (beginParse.success) return { type: "begin", data: beginParse.data };
  const errorParse = ResponseErrorSchema.safeParse(payload);
  if (errorParse.success) return { type: "error", data: errorParse.data };
  const chunkParse = LaboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkSchema.safeParse(payload);
  if (chunkParse.success) return { type: "chunk", data: chunkParse.data };
  return null;
}

function LogoMark({ className }: { className?: string }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="68 97 188 130" fill="currentColor" className={className}>
      <path d="M75.01,168.97h-3.01v-13.94h3.01c6.02,0,10.94-4.92,10.94-10.94v-17.64c0-13.67,11.21-24.88,24.88-24.88h7.66v13.94h-7.66c-6.02,0-10.94,4.92-10.94,10.94v17.64c0,6.97-3.01,13.4-7.66,17.91,4.65,4.51,7.66,10.94,7.66,17.91v17.64c0,6.01,4.92,10.94,10.94,10.94h7.66v13.94h-7.66c-13.67,0-24.88-11.21-24.88-24.88v-17.64c0-6.02-4.92-10.94-10.94-10.94Z"/>
      <path d="M231.77,162c-4.65-4.51-7.66-10.94-7.66-17.91v-17.64c0-6.02-4.92-10.94-10.94-10.94h-7.66v-13.94h7.66c13.67,0,24.88,11.21,24.88,24.88v17.64c0,6.02,4.92,10.94,10.94,10.94h3.01v13.94h-3.01c-6.02,0-10.94,4.92-10.94,10.94v17.64c0,13.67-11.21,24.88-24.88,24.88h-7.66v-13.94h7.66c6.02,0,10.94-4.92,10.94-10.94v-17.64c0-6.97,3.01-13.4,7.66-17.91Z"/>
      <path d="M123.38,155.18c.42-.49,1.29-1.22,2.59-2.17,1.3-.95,2.96-1.9,4.97-2.86,2.01-.95,4.36-1.78,7.04-2.49,2.68-.7,5.61-1.06,8.78-1.06s6.37.37,9.37,1.11c3,.74,5.66,1.96,7.99,3.65,2.33,1.69,4.18,3.91,5.56,6.67s2.06,6.14,2.06,10.16v31.43h-14.82l-1.27-6.03c-1.62,2.26-3.67,4.01-6.14,5.24-2.47,1.23-5.54,1.85-9.21,1.85-2.82,0-5.31-.41-7.46-1.22-2.15-.81-3.95-1.92-5.4-3.33-1.45-1.41-2.54-3.07-3.28-4.97-.74-1.91-1.11-3.95-1.11-6.14s.48-4.41,1.43-6.46c.95-2.04,2.47-3.83,4.55-5.34,2.08-1.52,4.78-2.73,8.1-3.65,3.32-.92,7.34-1.38,12.06-1.38h6.14v-.42c0-2.12-.76-3.77-2.28-4.98-1.52-1.2-4-1.8-7.46-1.8-1.55,0-3.05.21-4.5.64-1.45.42-2.75.92-3.92,1.48-1.16.57-2.17,1.15-3.02,1.75-.85.6-1.45,1.08-1.8,1.43l-8.99-11.11ZM155.34,177.4h-5.5c-3.53,0-5.96.65-7.3,1.96-1.34,1.31-2.01,2.77-2.01,4.39,0,1.2.46,2.33,1.38,3.39.92,1.06,2.4,1.59,4.45,1.59.85,0,1.8-.21,2.86-.63,1.06-.42,2.03-1.06,2.91-1.91.88-.85,1.64-1.92,2.28-3.23.63-1.3.95-2.8.95-4.5v-1.06Z"/>
      <path d="M178.94,133.8c0-1.55.28-3,.85-4.34.56-1.34,1.32-2.5,2.28-3.49.95-.99,2.08-1.76,3.39-2.33,1.3-.56,2.7-.85,4.18-.85s2.87.28,4.18.85c1.3.57,2.47,1.34,3.49,2.33,1.02.99,1.82,2.15,2.38,3.49.56,1.34.85,2.79.85,4.34s-.28,2.91-.85,4.29c-.57,1.38-1.36,2.56-2.38,3.54-1.02.99-2.19,1.78-3.49,2.38-1.31.6-2.7.9-4.18.9s-2.88-.3-4.18-.9c-1.31-.6-2.43-1.39-3.39-2.38-.95-.99-1.71-2.17-2.28-3.54-.57-1.38-.85-2.8-.85-4.29ZM180.95,147.77h17.46v51.86h-17.46v-51.86Z"/>
    </svg>
  );
}

function Wordmark({ className }: { className?: string }) {
  return (
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 818.93 324" fill="currentColor" className={className}>
      <path d="M86.3,161.32c0-5.52.99-10.74,2.98-15.64,1.98-4.9,4.79-9.17,8.41-12.79,3.62-3.62,7.92-6.48,12.9-8.58,4.98-2.1,10.47-3.15,16.46-3.15s11.48,1.05,16.46,3.15c4.98,2.1,9.26,4.96,12.84,8.58,3.58,3.62,6.36,7.88,8.35,12.79,1.99,4.9,2.98,10.12,2.98,15.64s-.99,10.61-2.98,15.47c-1.98,4.87-4.77,9.11-8.35,12.73-3.58,3.62-7.86,6.48-12.84,8.58-4.98,2.1-10.47,3.15-16.46,3.15s-11.48-1.05-16.46-3.15c-4.98-2.1-9.28-4.96-12.9-8.58-3.62-3.62-6.42-7.86-8.41-12.73-1.98-4.86-2.98-10.02-2.98-15.47ZM106.15,161.2c0,3.11.51,5.98,1.52,8.58,1.01,2.61,2.43,4.87,4.26,6.77,1.83,1.91,4.03,3.38,6.6,4.44,2.57,1.05,5.41,1.58,8.52,1.58s5.93-.53,8.46-1.58c2.53-1.05,4.71-2.53,6.54-4.44,1.83-1.91,3.25-4.16,4.26-6.77,1.01-2.61,1.52-5.47,1.52-8.58s-.51-5.97-1.52-8.58c-1.01-2.61-2.43-4.86-4.26-6.77-1.83-1.91-4.01-3.39-6.54-4.44-2.53-1.05-5.35-1.58-8.46-1.58s-5.95.53-8.52,1.58c-2.57,1.05-4.77,2.53-6.6,4.44-1.83,1.91-3.25,4.17-4.26,6.77-1.01,2.61-1.52,5.47-1.52,8.58Z"/>
      <path d="M176.9,115.55h19.15v30.59c4.83-3.27,10.27-4.9,16.35-4.9,4.13,0,8,.78,11.62,2.34,3.62,1.56,6.77,3.68,9.46,6.36,2.69,2.68,4.81,5.84,6.36,9.46,1.56,3.62,2.33,7.49,2.33,11.62s-.78,8-2.33,11.62c-1.56,3.62-3.68,6.77-6.36,9.46-2.68,2.69-5.84,4.81-9.46,6.36-3.62,1.56-7.49,2.34-11.62,2.34-3.58,0-6.95-.58-10.1-1.75-3.15-1.17-6.01-2.76-8.58-4.79l-.47,5.37h-16.35v-84.06ZM223.02,171.01c0-1.87-.37-3.6-1.11-5.19-.74-1.59-1.71-3.02-2.92-4.26-1.21-1.25-2.63-2.22-4.26-2.92-1.63-.7-3.39-1.05-5.25-1.05s-3.6.35-5.2,1.05c-1.6.7-3.02,1.67-4.26,2.92-1.25,1.25-2.22,2.67-2.92,4.26-.7,1.6-1.05,3.33-1.05,5.19s.35,3.6,1.05,5.19c.7,1.6,1.67,3.02,2.92,4.26,1.24,1.25,2.67,2.22,4.26,2.92,1.59.7,3.33,1.05,5.2,1.05s3.62-.35,5.25-1.05c1.63-.7,3.05-1.67,4.26-2.92,1.21-1.24,2.18-2.67,2.92-4.26.74-1.59,1.11-3.33,1.11-5.19Z"/>
      <path d="M271.94,142.4v60.01c0,3.74-.6,7.1-1.81,10.1-1.21,2.99-2.9,5.6-5.08,7.82-2.18,2.22-4.71,4.03-7.59,5.43-2.88,1.4-6.03,2.41-9.46,3.04l-5.37-12.96c2.96-1.56,5.37-3.29,7.24-5.2,1.87-1.91,2.8-4.65,2.8-8.23v-60.01h19.26ZM250.45,126.99c0-1.71.31-3.31.93-4.79.62-1.48,1.5-2.76,2.63-3.85,1.13-1.09,2.41-1.94,3.85-2.57,1.44-.62,2.98-.93,4.61-.93s3.17.31,4.61.93c1.44.63,2.69,1.48,3.74,2.57,1.05,1.09,1.89,2.37,2.51,3.85.62,1.48.93,3.07.93,4.79s-.31,3.21-.93,4.73c-.62,1.52-1.46,2.82-2.51,3.91-1.05,1.09-2.3,1.96-3.74,2.63-1.44.66-2.98.99-4.61.99s-3.17-.33-4.61-.99c-1.44-.66-2.72-1.54-3.85-2.63-1.13-1.09-2.01-2.39-2.63-3.91-.62-1.52-.93-3.09-.93-4.73Z"/>
      <path d="M336.61,193.19c-.86.78-2.1,1.63-3.74,2.57-1.63.93-3.54,1.79-5.72,2.57-2.18.78-4.55,1.42-7.12,1.93-2.57.5-5.18.76-7.82.76-4.44,0-8.52-.76-12.26-2.28-3.74-1.52-6.95-3.62-9.63-6.3-2.69-2.69-4.79-5.84-6.3-9.46-1.52-3.62-2.28-7.53-2.28-11.73s.76-8.11,2.28-11.73c1.52-3.62,3.62-6.77,6.3-9.46,2.68-2.68,5.9-4.79,9.63-6.3,3.74-1.52,7.82-2.28,12.26-2.28,4.75,0,9.01.76,12.78,2.28,3.77,1.52,6.99,3.6,9.63,6.25,2.64,2.65,4.69,5.78,6.13,9.4,1.44,3.62,2.16,7.53,2.16,11.73,0,1.01-.08,2.06-.23,3.15-.16,1.09-.31,2.1-.47,3.04l-40.75-.12c1.09,2.8,2.78,4.81,5.08,6.01,2.29,1.21,4.88,1.81,7.76,1.81,2.02,0,3.79-.19,5.31-.58,1.52-.39,2.8-.84,3.85-1.34,1.05-.51,1.89-1.01,2.51-1.52.62-.51,1.05-.88,1.28-1.11l9.34,12.73ZM323.65,166.45c-.16-2.57-1.13-4.77-2.92-6.6-1.79-1.83-4.36-2.74-7.71-2.74-2.96,0-5.51.76-7.65,2.28-2.14,1.52-3.6,3.87-4.38,7.06h22.65Z"/>
      <path d="M409.23,185.02c-.86,2.34-2.16,4.46-3.91,6.36-1.75,1.91-3.81,3.54-6.19,4.9-2.37,1.36-5,2.41-7.88,3.15-2.88.74-5.92,1.11-9.11,1.11-4.59,0-8.84-.72-12.73-2.16-3.89-1.44-7.24-3.48-10.04-6.13-2.8-2.64-5-5.78-6.6-9.4-1.6-3.62-2.39-7.57-2.39-11.85s.8-8.33,2.39-11.91c1.59-3.58,3.79-6.69,6.6-9.34,2.8-2.64,6.15-4.69,10.04-6.13,3.89-1.44,8.13-2.16,12.73-2.16,3.19,0,6.23.37,9.11,1.11,2.88.74,5.51,1.79,7.88,3.15,2.37,1.36,4.44,3,6.19,4.9,1.75,1.91,3.05,4.03,3.91,6.36l-14.94,7.35c-1.01-2.02-2.59-3.68-4.73-4.96-2.14-1.28-4.5-1.93-7.06-1.93-1.87,0-3.62.35-5.25,1.05-1.63.7-3.06,1.65-4.26,2.86-1.21,1.21-2.16,2.63-2.86,4.26-.7,1.64-1.05,3.39-1.05,5.25s.35,3.74,1.05,5.37c.7,1.64,1.65,3.08,2.86,4.32,1.21,1.25,2.63,2.22,4.26,2.92,1.64.7,3.39,1.05,5.25,1.05,2.57,0,4.92-.64,7.06-1.93,2.14-1.28,3.72-2.94,4.73-4.96l14.94,7.36Z"/>
      <path d="M421.84,142.4l4.09-15.18h15.18v15.18h15.06v15.18h-15.06v17.98c0,3.27.54,5.62,1.64,7.06,1.09,1.44,2.92,2.16,5.49,2.16,1.09,0,2.28-.12,3.56-.35,1.28-.24,2.74-.55,4.38-.94v15.18c-5.22,1.4-10.08,2.1-14.59,2.1-5.84,0-10.59-1.36-14.24-4.09-3.66-2.72-5.49-7.16-5.49-13.31v-25.8h-8.17v-15.18h8.17Z"/>
      <path d="M465.51,126.99c0-1.71.31-3.31.93-4.79.62-1.48,1.46-2.76,2.51-3.85,1.05-1.09,2.29-1.94,3.74-2.57,1.44-.62,2.98-.93,4.61-.93s3.17.31,4.61.93c1.44.63,2.72,1.48,3.85,2.57,1.13,1.09,2,2.37,2.63,3.85.62,1.48.93,3.07.93,4.79s-.31,3.21-.93,4.73c-.62,1.52-1.5,2.82-2.63,3.91-1.13,1.09-2.41,1.96-3.85,2.63-1.44.66-2.98.99-4.61.99s-3.17-.33-4.61-.99c-1.44-.66-2.68-1.54-3.74-2.63s-1.89-2.39-2.51-3.91c-.62-1.52-.93-3.09-.93-4.73ZM467.72,142.4h19.26v57.21h-19.26v-57.21Z"/>
      <path d="M517.23,142.4l9.69,36.54h.23l9.81-36.54h21.72l-20.2,57.21h-22.88l-20.08-57.21h21.72Z"/>
      <path d="M614.13,193.19c-.86.78-2.1,1.63-3.74,2.57-1.63.93-3.54,1.79-5.72,2.57-2.18.78-4.55,1.42-7.12,1.93-2.57.5-5.18.76-7.82.76-4.44,0-8.52-.76-12.26-2.28-3.74-1.52-6.95-3.62-9.63-6.3-2.69-2.69-4.79-5.84-6.3-9.46-1.52-3.62-2.28-7.53-2.28-11.73s.76-8.11,2.28-11.73c1.52-3.62,3.62-6.77,6.3-9.46,2.68-2.68,5.9-4.79,9.63-6.3s7.82-2.28,12.26-2.28c4.75,0,9.01.76,12.78,2.28,3.77,1.52,6.99,3.6,9.63,6.25,2.64,2.65,4.69,5.78,6.13,9.4,1.44,3.62,2.16,7.53,2.16,11.73,0,1.01-.08,2.06-.23,3.15-.16,1.09-.31,2.1-.47,3.04l-40.75-.12c1.09,2.8,2.78,4.81,5.08,6.01,2.29,1.21,4.88,1.81,7.76,1.81,2.02,0,3.79-.19,5.31-.58,1.52-.39,2.8-.84,3.85-1.34,1.05-.51,1.89-1.01,2.51-1.52.62-.51,1.05-.88,1.28-1.11l9.34,12.73ZM601.17,166.45c-.16-2.57-1.13-4.77-2.92-6.6-1.79-1.83-4.36-2.74-7.71-2.74-2.96,0-5.51.76-7.65,2.28-2.14,1.52-3.6,3.87-4.38,7.06h22.65Z"/>
      <path d="M652.19,122.79h22.77l28.6,76.82h-21.72l-4.55-12.26h-27.44l-4.55,12.26h-21.72l28.6-76.82ZM655.81,171.01h15.53l-7.59-25.33h-.23l-7.71,25.33Z"/>
      <path d="M732.63,199.61h-19.85v-76.82h19.85v76.82Z"/>
    </svg>
  );
}

const BADGE_LABELS: Record<Entry["kind"], string> = {
  "agent-completion": "agent completion",
  "execution": "function execution",
  "invention": "function invention",
  "laboratory": "laboratory",
};

function EntryView({ entry }: { entry: Entry }) {
  let content: React.ReactNode;

  if (entry.kind === "agent-completion") {
    content = <AgentCompletionView entry={entry} />;
  } else if (entry.kind === "invention") {
    content = <FunctionInventionRecursiveView entry={entry} />;
  } else if (entry.kind === "execution") {
    content = <FunctionExecutionView entry={entry} />;
  } else if (entry.error) {
    content = <pre style={{ color: "var(--error)", fontFamily: "var(--font-mono)", fontSize: 11, padding: 16 }}>{JSON.stringify(entry.error, null, 2)}</pre>;
  } else if (entry.chunk) {
    content = <pre style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--info-mid)", padding: 16 }}>{JSON.stringify(entry.chunk, null, 2)}</pre>;
  } else {
    content = <pre style={{ color: "var(--info-dim)", fontFamily: "var(--font-mono)", fontSize: 11, padding: 16 }}>{JSON.stringify(entry.request, null, 2)}</pre>;
  }

  return (
    <div className="entry-wrap">
      <span className={`entry-badge entry-badge-${entry.kind}`}>
        {BADGE_LABELS[entry.kind]}
      </span>
      {content}
    </div>
  );
}

function App() {
  const [entries, setEntries] = useState<Entry[]>([]);

  useEffect(() => {
    const isTauri = !!(window as any).__TAURI_INTERNALS__;
    if (!isTauri) {
      return replayMockEvents(setEntries);
    }

    const unlistenAgentCompletion = listen<unknown>("agent-completions", (event) => {
      const classified = classifyAgentCompletion(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "agent-completion" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "agent-completion")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "agent-completion") return e;
              const [merged] = e.chunk
                ? agentCompletionsResponseStreamingAgentCompletionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenExecution = listen<unknown>("functions-executions", (event) => {
      const classified = classifyFunctionExecution(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "execution",
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "execution")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "execution") return e;
              const [merged] = e.chunk
                ? functionsExecutionsResponseStreamingFunctionExecutionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenInvention = listen<unknown>("functions-inventions-recursive", (event) => {
      const classified = classifyFunctionInventionRecursive(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "invention",
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "invention")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "invention") return e;
              const [merged] = e.chunk
                ? functionsInventionsRecursiveResponseStreamingFunctionInventionRecursiveChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    const unlistenLaboratory = listen<unknown>("laboratories-executions", (event) => {
      const classified = classifyLaboratoryExecution(event.payload);
      if (!classified) return;

      setEntries((prev) => {
        switch (classified.type) {
          case "begin":
            return [...prev, {
              kind: "laboratory" as const,
              id: classified.data.id,
              request: classified.data,
              chunk: null,
              error: null,
            }];
          case "error": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id)) return prev;
            return prev.map((e) =>
              e.id === id ? { ...e, error: classified.data } : e
            );
          }
          case "chunk": {
            const id = classified.data.id;
            if (!prev.some((e) => e.id === id && e.kind === "laboratory")) return prev;
            return prev.map((e) => {
              if (e.id !== id || e.kind !== "laboratory") return e;
              const [merged] = e.chunk
                ? laboratoriesExecutionsResponseStreamingLaboratoryExecutionChunkMerged(e.chunk, classified.data)
                : [classified.data, true];
              return { ...e, chunk: merged };
            });
          }
        }
      });
    });

    // Signal the Rust backend that all listeners are registered.
    // Events are buffered on the Rust side until this resolves.
    Promise.all([unlistenAgentCompletion, unlistenExecution, unlistenInvention, unlistenLaboratory])
      .then(() => invoke("viewer_ready"));

    return () => {
      unlistenAgentCompletion.then((fn) => fn());
      unlistenExecution.then((fn) => fn());
      unlistenInvention.then((fn) => fn());
      unlistenLaboratory.then((fn) => fn());
    };
  }, []);

  return (
    <>
      <header className="viewer-header">
        <div className="viewer-logo">
          <LogoMark className="viewer-logo-mark" />
          <Wordmark className="viewer-wordmark" />
        </div>
      </header>
      <main className="viewer-main">
        {entries.length === 0 && <p className="viewer-empty">Waiting for requests...</p>}
        {entries.map((entry) => (
          <EntryView key={entry.id} entry={entry} />
        ))}
      </main>
    </>
  );
}

export default App;
