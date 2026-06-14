import type {
  AgentCompletionsMessageMessage,
  AgentCompletionsRequestAgentCompletionCreateParams,
  RemotePathCommitOptional,
} from "@objectiveai/sdk";

/**
 * Build an `AgentCompletionCreateParams` for the cli's
 * `objectiveai api agent completions post` endpoint.
 *
 * `agent` is a `RemotePathCommitOptional`, which the request's `agent`
 * field accepts directly.
 */
export function buildAgentCompletionRequest(
  agent: RemotePathCommitOptional,
  newMessages: AgentCompletionsMessageMessage[],
  continuation: string | null,
): AgentCompletionsRequestAgentCompletionCreateParams {
  return {
    agent: agent as AgentCompletionsRequestAgentCompletionCreateParams["agent"],
    messages: newMessages,
    continuation: continuation ?? null,
    stream: true,
  };
}
