// Test ESM import
import * as ObjectiveAI from "../dist/index.js";

console.log("Testing ESM...");

if (typeof ObjectiveAI !== "object" || ObjectiveAI === null) {
  console.log("  ✗ Failed to import module");
  process.exit(1);
}

const moduleExports = Object.keys(ObjectiveAI);

if (moduleExports.length === 0) {
  console.log("  ✗ Module has no exports");
  process.exit(1);
}

ObjectiveAI.EnsembleLlm.validate({
  model: "openai/gpt-5-nano",
  output_mode: "instruction",
});

console.log("ESM: PASSED");
