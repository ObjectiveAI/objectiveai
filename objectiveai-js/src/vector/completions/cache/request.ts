import z from "zod";
import {
  ModelSchema,
  FallbackModelsSchema,
} from "src/chat/completions/request/model";
import { MessagesSchema } from "src/chat/completions/request/message";
import { ToolsSchema } from "src/chat/completions/request/tool";
import { VectorResponsesSchema } from "../request/vector_response";

export const CacheVoteRequestSchema = z
  .object({
    model: ModelSchema,
    models: FallbackModelsSchema.optional().nullable(),
    messages: MessagesSchema,
    tools: ToolsSchema.optional().nullable(),
    responses: VectorResponsesSchema,
  })
  .describe(
    "Parameters for requesting a cached vote from the ObjectiveAI votes cache.",
  );
export type CacheVoteRequest = z.infer<typeof CacheVoteRequestSchema>;
