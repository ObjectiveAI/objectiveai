// Test ESM import
import { wasmAgentValidateAgent } from "../dist/index.js";

console.log("Testing ESM...");

wasmAgentValidateAgent({
  upstream: "openrouter",
  model: "openai/gpt-5-nano",
  output_mode: "instruction",
});

console.log("ESM: PASSED");
