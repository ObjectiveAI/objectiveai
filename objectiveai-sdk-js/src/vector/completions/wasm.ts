import { vectorResponseId } from "../../wasm/loader.js";
import type { AgentCompletionsMessageRichContent } from "../../agent/completions/message/richContent";

export function wasmVectorCompletionsVectorResponseId(response: AgentCompletionsMessageRichContent): string {
  return vectorResponseId(response);
}
