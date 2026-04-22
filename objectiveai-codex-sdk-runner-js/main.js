#!/usr/bin/env node
/**
 * ObjectiveAI Codex SDK Runner (JavaScript).
 *
 * Runs the official OpenAI Codex SDK (@openai/codex-sdk) and streams
 * ThreadEvents to stdout as NDJSON. Designed to be spawned as a
 * subprocess by objectiveai-api.
 *
 * Mirrors the CLI surface of objectiveai-codex-sdk-runner-py.
 *
 * Authentication is inherited from ~/.codex/auth.json (written by
 * `codex login`). The SDK shells out to a `codex` binary that reads
 * that file — we do nothing special for auth. The codex binary can be
 * supplied via:
 *   1. --codex-bin CLI arg
 *   2. CODEX_BIN environment variable
 *   3. `codex` on PATH
 */

import { parseArgs } from "node:util";
import * as fs from "node:fs";
import { execSync } from "node:child_process";
import { Codex } from "@openai/codex-sdk";

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

const parsed = parseArgs({
  options: {
    model: { type: "string" },
    input: { type: "string" },
    effort: { type: "string" },
    sandbox: { type: "string" },
    "approval-policy": { type: "string", default: "never" },
    cwd: { type: "string" },
    "additional-directory": { type: "string", multiple: true },
    "output-schema": { type: "string" },
    resume: { type: "string" },
    "codex-bin": { type: "string" },
    "base-url": { type: "string" },
    "api-key": { type: "string" },
    "skip-git-repo-check": { type: "boolean" },
    "no-skip-git-repo-check": { type: "boolean" },
    "network-access-enabled": { type: "boolean" },
    "no-network-access-enabled": { type: "boolean" },
    "web-search-enabled": { type: "boolean" },
    "no-web-search-enabled": { type: "boolean" },
    "web-search-mode": { type: "string" },
    help: { type: "boolean", short: "h" },
  },
  strict: true,
});
const args = parsed.values;

function printHelp() {
  process.stdout.write(
    [
      "Usage: objectiveai-codex-sdk-runner-js [options]",
      "",
      "Run the OpenAI Codex SDK and stream ThreadEvents to stdout as NDJSON.",
      "",
      "Required:",
      "  --input <JSON>            Turn input: a JSON array of input items or a plain string.",
      "                            Items: {\"type\":\"text\",\"text\":...} | {\"type\":\"local_image\",\"path\":...}",
      "",
      "Optional (unless --resume is given, --model is also required):",
      "  --model <id>              Codex model identifier (e.g. gpt-5).",
      "  --effort <level>          minimal | low | medium | high | xhigh",
      "  --sandbox <mode>          read-only | workspace-write | danger-full-access",
      "  --approval-policy <mode>  never | on-request | on-failure | untrusted  [default: never]",
      "  --cwd <path>              Working directory for the thread.",
      "  --additional-directory <path>   Repeatable: extra directories the sandbox may access.",
      "  --output-schema <JSON>    Structured-output JSON schema.",
      "  --resume <thread_id>      Resume a previously started thread.",
      "  --codex-bin <path>        Path to the codex binary.",
      "  --base-url <url>          Override Codex API base URL.",
      "  --api-key <key>           Override API key (bypasses ChatGPT subscription auth).",
      "  --skip-git-repo-check / --no-skip-git-repo-check",
      "  --network-access-enabled / --no-network-access-enabled",
      "  --web-search-enabled / --no-web-search-enabled",
      "  --web-search-mode <mode>  disabled | cached | live",
      "  -h, --help                Show this help.",
      "",
    ].join("\n"),
  );
}

if (args.help) {
  printHelp();
  process.exit(0);
}

// Tri-state: --foo / --no-foo / absent (undefined).
function triState(onKey, offKey) {
  if (args[onKey]) return true;
  if (args[offKey]) return false;
  return undefined;
}

const skipGitRepoCheck = triState("skip-git-repo-check", "no-skip-git-repo-check");
const networkAccessEnabled = triState("network-access-enabled", "no-network-access-enabled");
const webSearchEnabled = triState("web-search-enabled", "no-web-search-enabled");

if (!args.input) {
  process.stderr.write("Error: --input is required\n");
  process.exit(1);
}
if (!args.resume && !args.model) {
  process.stderr.write("Error: --model is required unless --resume is given\n");
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Input parsing
// ---------------------------------------------------------------------------

function parseInput(raw) {
  const value = JSON.parse(raw);
  if (typeof value === "string") {
    return value;
  }
  if (!Array.isArray(value)) {
    throw new Error("--input must be a JSON array of input items or a plain string");
  }
  const out = [];
  value.forEach((item, idx) => {
    if (!item || typeof item !== "object" || !item.type) {
      throw new Error(`--input[${idx}] must be an object with a "type" field`);
    }
    if (item.type === "text") {
      if (typeof item.text !== "string") {
        throw new Error(`--input[${idx}] missing required field: text`);
      }
      out.push({ type: "text", text: item.text });
    } else if (item.type === "local_image") {
      if (typeof item.path !== "string") {
        throw new Error(`--input[${idx}] missing required field: path`);
      }
      out.push({ type: "local_image", path: item.path });
    } else {
      throw new Error(`--input[${idx}] has unknown type: ${JSON.stringify(item.type)}`);
    }
  });
  return out;
}

// ---------------------------------------------------------------------------
// Codex binary resolution
// ---------------------------------------------------------------------------

function resolveCodexBin(explicit) {
  if (explicit) return explicit;
  if (process.env.CODEX_BIN) return process.env.CODEX_BIN;
  // Cross-platform PATH lookup for `codex`.
  try {
    const cmd = process.platform === "win32" ? "where codex" : "command -v codex";
    const result = execSync(cmd, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
    if (result) {
      // `where` may return multiple lines; take the first.
      const first = result.split(/\r?\n/)[0].trim();
      if (first && fs.existsSync(first)) return first;
    }
  } catch {
    /* not found */
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

function only(obj) {
  const out = {};
  for (const [k, v] of Object.entries(obj)) {
    if (v !== undefined && v !== null) out[k] = v;
  }
  return out;
}

async function main() {
  const input = parseInput(args.input);
  const outputSchema = args["output-schema"] ? JSON.parse(args["output-schema"]) : undefined;

  const codex = new Codex(
    only({
      codexPathOverride: resolveCodexBin(args["codex-bin"]),
      baseUrl: args["base-url"],
      apiKey: args["api-key"],
    }),
  );

  const threadOptions = only({
    model: args.model,
    sandboxMode: args.sandbox,
    approvalPolicy: args["approval-policy"],
    workingDirectory: args.cwd,
    additionalDirectories: args["additional-directory"],
    modelReasoningEffort: args.effort,
    skipGitRepoCheck,
    networkAccessEnabled,
    webSearchEnabled,
    webSearchMode: args["web-search-mode"],
  });

  const thread = args.resume
    ? codex.resumeThread(args.resume, threadOptions)
    : codex.startThread(threadOptions);

  const turnOptions = only({ outputSchema });
  const streamed = await thread.runStreamed(input, turnOptions);

  let exitCode = 0;
  for await (const event of streamed.events) {
    process.stdout.write(JSON.stringify(event) + "\n");
    if (event && (event.type === "turn.failed" || event.type === "error")) {
      exitCode = 1;
    }
  }
  process.exit(exitCode);
}

main().catch((e) => {
  const name = e && e.name ? e.name : "Error";
  const msg = e && e.message ? e.message : String(e);
  process.stderr.write(`${name}: ${msg}\n`);
  process.exit(1);
});
