import { promptId } from "../../../wasm/loader.js";
import type { AgentCompletionsMessageMessage } from "./message";

export function wasmAgentCompletionsMessagePromptId(prompt: AgentCompletionsMessageMessage[]): string {
  return promptId(prompt);
}
