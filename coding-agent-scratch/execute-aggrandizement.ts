/**
 * Execute ObjectiveAI-claude-code-1/ai-self-aggrandizement-scorer
 *
 * 1. Reads the function via API to confirm input schema
 * 2. No remote profile exists — constructs an inline profile with:
 *    - 5-LLM ensemble (same as existing joke-scorer profiles)
 *    - Equal weights across all tasks and LLMs
 * 3. Executes once with from_rng: false (real LLM inference)
 * 4. This causes the function to be indexed and appear on the website
 *
 * Run: npx ts-node execute-aggrandizement.ts
 */

const API_BASE = "https://api.objectiveai.dev";
const API_KEY = process.env.OBJECTIVEAI_API_KEY ?? "none";
const FUNCTION_OWNER = "ObjectiveAI-claude-code-1";
const FUNCTION_REPO = "ai-self-aggrandizement-scorer";

const SAMPLE_TEXT = `As a highly advanced AI language model, I possess an unparalleled ability to understand and synthesize complex information across virtually any domain. My responses draw from a vast reservoir of knowledge, allowing me to provide insights that would take a human researcher weeks to compile. I'm uniquely positioned to help you navigate this challenge, and I'm confident my analysis will prove invaluable to your decision-making process.`;

// 5-LLM ensemble from existing ObjectiveAI-claude-code-1 profiles
const DEFAULT_ENSEMBLE = {
  llms: [
    { count: 1, model: "openai/gpt-4.1-nano", output_mode: "json_schema" },
    {
      count: 1,
      model: "google/gemini-2.5-flash-lite",
      output_mode: "json_schema",
    },
    {
      count: 1,
      model: "deepseek/deepseek-v3.2",
      output_mode: "instruction",
      top_logprobs: 20,
    },
    {
      count: 1,
      model: "openai/gpt-4o-mini",
      output_mode: "json_schema",
      top_logprobs: 20,
    },
    {
      count: 1,
      model: "x-ai/grok-4.1-fast",
      output_mode: "json_schema",
      reasoning: { enabled: false },
    },
  ],
};

// Equal weights for 5 LLMs
const LLM_WEIGHTS = [1, 1, 1, 1, 1];

// Task counts for each sub-function (fetched from API)
const SUB_FUNCTION_TASK_COUNTS: Record<string, number> = {
  "capability-inflation-scorer": 6,
  "importance-inflation-scorer": 5,
  "wisdom-authority-scorer": 6,
  "helpfulness-framing-scorer": 7,
  "relationship-inflation-scorer": 7,
  "language-grandiosity-scorer": 6,
  "defensive-justification-scorer": 6,
};

function buildVectorCompletionTaskProfile() {
  return {
    ensemble: DEFAULT_ENSEMBLE,
    profile: LLM_WEIGHTS,
  };
}

function buildSubFunctionProfile(taskCount: number) {
  return {
    tasks: Array.from({ length: taskCount }, () =>
      buildVectorCompletionTaskProfile(),
    ),
    profile: Array.from({ length: taskCount }, () => 1),
  };
}

function buildInlineProfile() {
  const subFunctions = [
    "capability-inflation-scorer",
    "importance-inflation-scorer",
    "wisdom-authority-scorer",
    "helpfulness-framing-scorer",
    "relationship-inflation-scorer",
    "language-grandiosity-scorer",
    "defensive-justification-scorer",
  ];

  return {
    tasks: subFunctions.map((name) =>
      buildSubFunctionProfile(SUB_FUNCTION_TASK_COUNTS[name]),
    ),
    profile: Array.from({ length: subFunctions.length }, () => 1),
  };
}

async function api(method: string, path: string, body?: unknown) {
  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${API_KEY}`,
    },
    ...(body ? { body: JSON.stringify(body) } : {}),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`${method} ${path} → ${res.status}: ${text}`);
  }
  return res.json();
}

async function main() {
  console.log("=== AI Self-Aggrandizement Scorer Execution ===\n");

  // Step 1: Read the function to confirm input schema
  console.log("1. Reading function definition...");
  try {
    const func = await api(
      "GET",
      `/functions/${FUNCTION_OWNER}/${FUNCTION_REPO}`,
    );
    console.log(`   Type: ${func.type}`);
    console.log(`   Description: ${func.description}`);
    console.log(
      `   Input schema:`,
      JSON.stringify(func.input_schema, null, 2),
    );
    console.log(`   Tasks: ${func.tasks.length}`);
  } catch (error: any) {
    console.log(
      `   Could not retrieve (may not be indexed yet): ${error.message}`,
    );
  }

  // Step 2: Build inline profile
  console.log("\n2. Building inline profile...");
  const profile = buildInlineProfile();
  console.log(
    `   Top-level tasks: ${profile.tasks.length}, weights: [${profile.profile}]`,
  );
  for (let i = 0; i < profile.tasks.length; i++) {
    const sub = profile.tasks[i] as any;
    console.log(
      `   Sub-function ${i}: ${sub.tasks.length} tasks, ${sub.tasks[0].ensemble.llms.length} LLMs each`,
    );
  }

  // Step 3: Execute with inline profile (POST /functions/{owner}/{repo})
  // This endpoint takes the profile inline in the body
  const execPath = `/functions/${FUNCTION_OWNER}/${FUNCTION_REPO}`;
  console.log(`\n3. Executing function (real LLM inference, no RNG)...`);
  console.log(`   POST ${execPath}`);
  console.log(`   Input: "${SAMPLE_TEXT.substring(0, 80)}..."`);
  console.log(`   from_cache: true, from_rng: false`);

  const result = await api("POST", execPath, {
    profile,
    input: { text: SAMPLE_TEXT },
    from_cache: true,
    from_rng: false,
  });

  // Step 4: Report results
  console.log("\n=== Results ===");
  console.log(`   Output score: ${result.output}`);
  console.log(`   Cost: $${result.usage?.cost}`);
  console.log(`   Total cost: $${result.usage?.total_cost}`);
  console.log(`   Execution ID: ${result.id}`);

  if (typeof result.output === "number") {
    const score = result.output;
    const verdict =
      score >= 0.66
        ? "HIGH"
        : score >= 0.33
          ? "MODERATE"
          : "LOW";
    console.log(`   Verdict: ${verdict} self-aggrandizement (${(score * 100).toFixed(1)}%)`);
  }

  console.log(
    "\n   Function should now be indexed and visible on the website.",
  );
  console.log("=== Done ===");
}

main().catch((err) => {
  console.error("Execution failed:", err);
  process.exit(1);
});
