/**
 * SDK Testing Script
 *
 * Use this folder to test SDK calls and explore API behavior.
 * Run with: npx ts-node test-sdk.ts
 *
 * Prerequisites:
 * - npm install objectiveai typescript ts-node
 * - Set OBJECTIVEAI_API_KEY environment variable (or use anonymous)
 */

import { ObjectiveAI, Auth, Functions, EnsembleLlm } from "objectiveai";

async function main() {
  // Initialize client (uses OBJECTIVEAI_API_KEY from env, or anonymous)
  const client = new ObjectiveAI({
    apiKey: process.env.OBJECTIVEAI_API_KEY ?? "none",
    apiBase: "https://api.objectiveai.dev",
  });

  console.log("=== ObjectiveAI SDK Test ===\n");

  // Test 1: Get credits (shows remaining balance for anonymous/authenticated users)
  try {
    console.log("1. Fetching credits...");
    const credits = await Auth.Credits.retrieve(client);
    console.log(`   Credits: ${credits.credits}`);
  } catch (error) {
    console.log(`   Error: ${error}`);
  }

  // Test 2: List functions
  try {
    console.log("\n2. Listing functions...");
    const functions = await Functions.list(client, { limit: 5 });
    console.log(`   Found ${functions.length} functions:`);
    for (const fn of functions) {
      console.log(`   - ${fn.owner}/${fn.repository}`);
    }
  } catch (error) {
    console.log(`   Error: ${error}`);
  }

  // Test 3: Execute a function (using cache and RNG for free execution)
  // Note: Both function and profile are required. Use listPairs() to find valid pairs.
  try {
    console.log("\n3. Listing function-profile pairs...");
    const pairs = await Functions.listPairs(client);
    if (pairs.data.length > 0) {
      const pair = pairs.data[0];
      console.log(`   Executing ${pair.function.owner}/${pair.function.repository}...`);
      const result = await Functions.Executions.create(
        client,
        pair.function,
        pair.profile,
        {
          input: { text: "hello world" },
          from_cache: true,
          from_rng: true,
        }
      );
      console.log(`   Output: ${result.output}`);
      console.log(`   Cost: $${result.usage.cost}`);
    } else {
      console.log("   No function-profile pairs found");
    }
  } catch (error) {
    console.log(`   Error: ${error}`);
  }

  console.log("\n=== Tests Complete ===");
}

main().catch(console.error);
