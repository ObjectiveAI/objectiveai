import OpenAI from "openai";
import z from "zod";
import { VectorCompletion } from "./response/unary/vector_completion";
import { Stream } from "openai/streaming";
import {
  VectorCompletionCreateParams,
  VectorCompletionCreateParamsStreaming,
} from "./request/vector_completion_create_params";
import { VectorCompletionChunk } from "./response/streaming/vector_completion_chunk";
import { VotesSchema } from "./response/vote";

export async function create(
  openai: OpenAI,
  body: VectorCompletionCreateParamsStreaming,
  options?: OpenAI.RequestOptions,
): Promise<Stream<VectorCompletionChunk>>;
export async function create(
  openai: OpenAI,
  body: VectorCompletionCreateParams,
  options?: OpenAI.RequestOptions,
): Promise<VectorCompletion>;
export async function create(
  openai: OpenAI,
  body: VectorCompletionCreateParams,
  options?: OpenAI.RequestOptions,
): Promise<Stream<VectorCompletionChunk> | VectorCompletion> {
  const response = await openai.post("/vector/completions", {
    body,
    stream: body.stream ?? false,
    ...options,
  });
  return response as Stream<VectorCompletionChunk> | VectorCompletion;
}

export const RetrieveSchema = z
  .object({
    data: VotesSchema.optional().nullable(),
  })
  .describe("Response containing votes from a historical vector completion.");
export type Retrieve = z.infer<typeof RetrieveSchema>;

export async function retrieve(
  openai: OpenAI,
  id: string,
  options?: OpenAI.RequestOptions,
): Promise<Retrieve> {
  const response = await openai.get(`/vector/completions/${id}`, options);
  return response as Retrieve;
}
