const { ObjectiveAI, Functions } = require("objectiveai");

const client = new ObjectiveAI({ apiKey: "none", apiBase: "https://api.objectiveai.dev" });

async function test() {
  try {
    // Test 1: listPairs
    const pairs = await Functions.listPairs(client);
    const mvr = pairs.data.find(p => p.function.repository === "market-viability-ranker");
    console.log("From listPairs:");
    console.log("  Has input_schema:", !!mvr?.function?.input_schema);
    
    // Test 2: retrieve
    const details = await Functions.retrieve(client, "ObjectiveAI-claude-code-1", "market-viability-ranker", null);
    console.log("\nFrom retrieve:");
    console.log("  Has input_schema:", !!details.input_schema);
    console.log("  Schema type:", details.input_schema?.type);
    console.log("  Schema keys:", details.input_schema ? Object.keys(details.input_schema) : "none");
  } catch (err) {
    console.error("Error:", err.message);
  }
}

test();
