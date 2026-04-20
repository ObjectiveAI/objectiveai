/**
 * Execute the YC Application Scorer function.
 *
 * Prerequisites:
 *   - function.json must be pushed to a GitHub repo first
 *   - Set OBJECTIVEAI_API_KEY env var
 *   - Set YC_SCORER_OWNER and YC_SCORER_REPO env vars (defaults below)
 *
 * Run: OBJECTIVEAI_API_KEY=apk... npx ts-node execute-yc-scorer.ts
 */

const API_BASE = "https://api.objectiveai.dev";
const API_KEY = process.env.OBJECTIVEAI_API_KEY ?? "none";
const FUNCTION_OWNER =
  process.env.YC_SCORER_OWNER ?? "ObjectiveAI-claude-code-1";
const FUNCTION_REPO =
  process.env.YC_SCORER_REPO ?? "yc-application-scorer";

// 5-LLM ensemble (same as other ObjectiveAI-claude-code-1 profiles)
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

const LLM_WEIGHTS = [1, 1, 1, 1, 1];

// 6 vector.completion tasks → 6 task profiles, equal top-level weights
function buildInlineProfile() {
  const TASK_COUNT = 6;
  return {
    tasks: Array.from({ length: TASK_COUNT }, () => ({
      ensemble: DEFAULT_ENSEMBLE,
      profile: LLM_WEIGHTS,
    })),
    profile: Array.from({ length: TASK_COUNT }, () => 1),
  };
}

// Sample ObjectiveAI YC application (real company from this session)
const SAMPLE_INPUT = {
  company_name: "Objective Artificial Intelligence, Inc.",
  company_description_short: "Score everything. Rank everything. Simulate anyone.",
  company_url: "https://objectiveai.dev",
  product_link: "https://objectiveai.dev",
  what_company_makes:
    "ObjectiveAI is a REST API platform for scoring, ranking, and simulating preferences using ensembles of LLMs. Instead of asking one model for an answer, it uses multiple LLMs with explicit weights to produce structured numeric outputs. Developers define scoring functions as composable pipelines of vector completions, where each LLM votes on responses and votes are combined using learned weights. The platform supports probabilistic voting via logprobs, profile training to learn optimal weights, and content-addressed immutable definitions.",
  location: "San Francisco, USA / San Francisco, USA",
  location_reasoning:
    "Core AI/ML talent pool, proximity to YC network and partners, existing investor relationships in the Bay Area.",
  founders: [
    {
      founder_name: "Ronald Riggles",
      founder_title: "CEO & Co-Founder",
      founder_bio:
        "CEO & Co-Founder. Built the entire platform: Rust SDK, API server, WASM bindings, TypeScript SDK, and backend infrastructure. Deep expertise in LLM orchestration, probabilistic systems, and API design.",
      founder_equity_pct: 50,
    },
    {
      founder_name: "Maya Gore",
      founder_title: "COO & Co-Founder",
      founder_bio:
        "COO & Co-Founder. Handles UI/UX, branding, and frontend implementation. Worked alongside Claude Code to build the web interface. Focused on visual experience and product clarity.",
      founder_equity_pct: 50,
    },
  ],
  who_writes_code:
    "Ronald writes all backend code — the Rust core SDK, API server, WASM bindings, TypeScript SDK. Maya works with Claude Code on the web frontend. No non-founder technical work.",
  looking_for_cofounder: false,
  how_far_along:
    "Fully launched. Production API at api.objectiveai.dev serving real traffic. Complete TypeScript SDK published on npm. Web interface live with function browsing, execution UI, API key management, credit purchasing via Stripe, and 32-page API docs. Multiple scoring functions deployed and indexed.",
  how_long_working:
    "Ronald has been working on this for approximately 8 months, full-time for the last 5 months. Maya joined as Co-Founder and has been working on design and frontend for several months.",
  tech_stack:
    "Rust (core SDK, API server, WASM bindings), TypeScript (JS SDK, Next.js web app), Google Cloud Run for deployment, Stripe for payments. LLMs accessed via OpenRouter. Content-addressed IDs using XXHash3-128. Client-side validation via WASM in browser.",
  people_using_product: true,
  has_revenue: true,
  why_this_idea:
    "Current LLM applications ask one model for an answer and hope for the best. This is fragile — outputs vary by model, prompt, and temperature. ObjectiveAI treats LLM outputs as votes in an ensemble, producing calibrated numeric scores instead of text. The founder has deep experience with distributed systems and saw that the ensemble approach used in traditional ML (random forests, boosting) had no equivalent for LLMs. The market needs structured, reliable outputs from LLMs — not more chatbots.",
  competitors:
    "No direct competitor does ensemble LLM voting with learned weights for structured scoring. Adjacent: LangChain/LlamaIndex (orchestration, not scoring), Braintrust/Humanloop (evals, not production scoring), OpenRouter (model routing, not ensembles). We understand that the value is in the weights, not the models — same ensemble behaves differently with different learned weights, which no competitor offers.",
  how_make_money:
    "Usage-based API pricing. Each function execution incurs a cost based on upstream LLM usage plus ObjectiveAI margin. Current pricing covers compute costs with healthy margins. Revenue scales linearly with usage. TAM: every application that needs a score, rank, or preference — content moderation, recommendation, hiring, grading, pricing. Conservative estimate: $50M ARR achievable with 1% of the LLM evaluation market.",
  company_category: "B2B / API / AI Infrastructure",
  legal_entity: "Objective Artificial Intelligence, Inc. - Delaware C Corp",
  equity_breakdown: "Ronald Riggles (CEO & Co-Founder): 50%, Maya Gore (COO & Co-Founder): 50%",
  investment_taken: false,
  currently_fundraising: false,
  why_yc:
    "YC's network would help with enterprise sales motion and hiring the first engineers. The batch structure provides accountability and deadlines. Several YC companies in the AI infrastructure space could be early design partners.",
};

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
  console.log("=== YC Application Scorer Execution ===\n");

  // Step 1: Read the function
  console.log("1. Reading function definition...");
  try {
    const func = await api(
      "GET",
      `/functions/${FUNCTION_OWNER}/${FUNCTION_REPO}`,
    );
    console.log(`   Type: ${func.type}`);
    console.log(`   Tasks: ${func.tasks.length}`);
    console.log(`   Required fields: ${func.input_schema.required?.join(", ")}`);
  } catch (error: any) {
    console.log(`   Could not retrieve (may not be indexed yet): ${error.message}`);
    console.log("   Will attempt execution anyway (server fetches from GitHub)...");
  }

  // Step 2: Build inline profile
  console.log("\n2. Building inline profile...");
  const profile = buildInlineProfile();
  console.log(
    `   ${profile.tasks.length} task profiles, ${profile.tasks[0].ensemble.llms.length} LLMs each, equal weights`,
  );

  // Step 3: Execute
  const execPath = `/functions/${FUNCTION_OWNER}/${FUNCTION_REPO}`;
  console.log(`\n3. Executing (real LLM inference, no RNG)...`);
  console.log(`   POST ${execPath}`);
  console.log(`   Company: ${SAMPLE_INPUT.company_name}`);
  console.log(`   Founders: ${SAMPLE_INPUT.founders.map((f) => f.founder_name).join(", ")}`);

  const result = await api("POST", execPath, {
    profile,
    input: SAMPLE_INPUT,
    from_cache: true,
    from_rng: false,
  });

  // Step 4: Results
  console.log("\n=== Results ===");
  console.log(`   Output score: ${result.output}`);
  console.log(`   Cost: $${result.usage?.cost}`);
  console.log(`   Total cost: $${result.usage?.total_cost}`);
  console.log(`   Execution ID: ${result.id}`);

  if (typeof result.output === "number") {
    const pct = (result.output * 100).toFixed(1);
    const verdict =
      result.output >= 0.75
        ? "STRONG ACCEPT"
        : result.output >= 0.5
          ? "LEAN ACCEPT"
          : result.output >= 0.33
            ? "BORDERLINE"
            : "LEAN REJECT";
    console.log(`   Verdict: ${verdict} (${pct}%)`);
  }

  // Show per-task breakdown if available
  if (result.tasks && Array.isArray(result.tasks)) {
    const dimensions = [
      "Founding Team",
      "Progress & Traction",
      "Idea & Market",
      "Business Model",
      "Competitive Positioning",
      "Clarity & Conviction",
    ];
    console.log("\n   Per-dimension breakdown:");
    for (let i = 0; i < result.tasks.length; i++) {
      const task = result.tasks[i];
      const scores = task.scores;
      if (scores && scores.length === 5) {
        const taskScore =
          scores[0] * 1.0 +
          scores[1] * 0.75 +
          scores[2] * 0.5 +
          scores[3] * 0.25 +
          scores[4] * 0.0;
        console.log(
          `   ${dimensions[i] ?? "Task " + i}: ${(taskScore * 100).toFixed(1)}%`,
        );
      }
    }
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
