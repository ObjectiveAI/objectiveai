import { ObjectiveAI, type RequestOptions } from "../../../client";
import type { RemotePathCommitOptional } from "../../../remotePathCommitOptional";
import type { FunctionsInventionsPromptsListPromptsRequest } from "./listPromptsRequest";
import type { FunctionsInventionsPromptsListPromptResponse } from "./listPromptResponse";
import type { FunctionsInventionsPromptsGetPromptResponse } from "./getPromptResponse";
import type { FunctionsInventionsPromptsUsagePromptResponse } from "./usagePromptResponse";

export function functionsInventionsPromptsListPrompts(
  client: ObjectiveAI,
  params: FunctionsInventionsPromptsListPromptsRequest,
  options?: RequestOptions,
): Promise<FunctionsInventionsPromptsListPromptResponse> {
  return client.post_unary<FunctionsInventionsPromptsListPromptResponse>("functions/inventions/prompts/list", params, options);
}

export function functionsInventionsPromptsGetPrompt(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsInventionsPromptsGetPromptResponse> {
  return client.post_unary<FunctionsInventionsPromptsGetPromptResponse>("functions/inventions/prompts", params, options);
}

export function functionsInventionsPromptsGetPromptUsage(
  client: ObjectiveAI,
  params: RemotePathCommitOptional,
  options?: RequestOptions,
): Promise<FunctionsInventionsPromptsUsagePromptResponse> {
  return client.post_unary<FunctionsInventionsPromptsUsagePromptResponse>("functions/inventions/prompts/usage", params, options);
}
