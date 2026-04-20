#!/usr/bin/env node
/**
 * ObjectiveAI Claude Agent SDK Runner (JavaScript).
 *
 * Runs the Claude Agent SDK and streams JSONL events to stdout.
 * Designed to be spawned as a subprocess by objectiveai-api.
 *
 * All parameters are accepted as CLI arguments (same API as the Python runner).
 *
 * The @anthropic-ai/claude-agent-sdk package is NOT bundled (proprietary
 * license). At runtime we resolve it from the host system — global npm,
 * Claude Code's local install, or any location Node's module resolution finds.
 */

"use strict";

const { parseArgs } = require("node:util");
const { createRequire } = require("node:module");
const path = require("node:path");
const fs = require("node:fs");
const { execSync } = require("node:child_process");

// ---------------------------------------------------------------------------
// SDK resolution
// ---------------------------------------------------------------------------
// Try several candidate paths to find @anthropic-ai/claude-agent-sdk.

function resolveSdk() {
  const sdkName = "@anthropic-ai/claude-agent-sdk";
  const candidates = [];

  // 1. Global npm root
  try {
    const globalRoot = execSync("npm root -g", { encoding: "utf8" }).trim();
    if (globalRoot) candidates.push(path.join(globalRoot, sdkName));
  } catch { /* ignore */ }

  // 2. Claude Code local install
  const home = process.env.HOME || process.env.USERPROFILE;
  if (home) {
    candidates.push(path.join(home, ".claude", "local", "node_modules", sdkName));
  }

  // 3. Current working directory's node_modules
  candidates.push(path.join(process.cwd(), "node_modules", sdkName));

  // 4. NODE_PATH
  if (process.env.NODE_PATH) {
    for (const p of process.env.NODE_PATH.split(path.delimiter)) {
      candidates.push(path.join(p, sdkName));
    }
  }

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      // Create a require anchored at the SDK's package.json so its internal
      // requires resolve correctly too.
      const pkgPath = path.join(candidate, "package.json");
      if (fs.existsSync(pkgPath)) {
        const req = createRequire(pkgPath);
        return req(candidate);
      }
    }
  }

  throw new Error(
    "@anthropic-ai/claude-agent-sdk not found. Install it globally: " +
    "npm install -g @anthropic-ai/claude-agent-sdk"
  );
}

const { query } = resolveSdk();

// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------

const { values: args } = parseArgs({
  options: {
    model: { type: "string" },
    message: { type: "string" },
    "system-prompt": { type: "string" },
    effort: { type: "string" },
    "thinking-disabled": { type: "boolean", default: false },
    "mcp-servers": { type: "string" },
    resume: { type: "string" },
    "user-agent": { type: "string" },
    "rate-limit-max-retries": { type: "string" },
  },
  strict: true,
});

if (!args.model || !args.message) {
  process.stderr.write("--model and --message are required\n");
  process.exit(1);
}

const rateLimitMaxRetries = parseInt(args["rate-limit-max-retries"] || "10", 10);

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

delete process.env.CLAUDECODE;

if (args["user-agent"]) {
  process.env.CLAUDE_AGENT_SDK_CLIENT_APP = args["user-agent"];
}

const message = JSON.parse(args.message);
const mcpServers = args["mcp-servers"] ? JSON.parse(args["mcp-servers"]) : {};

// Build options (mirrors Python's ClaudeAgentOptions)
const opts = {
  model: args.model,
  tools: [],
  includePartialMessages: true,
  permissionMode: "bypassPermissions",
};

if (args["system-prompt"]) {
  opts.systemPrompt = args["system-prompt"];
}
if (args.effort) {
  opts.effort = args.effort;
}
if (args["thinking-disabled"]) {
  opts.thinking = { type: "disabled" };
}
if (Object.keys(mcpServers).length > 0) {
  opts.mcpServers = mcpServers;
}
if (args.resume) {
  opts.resume = args.resume;
}

async function* messages() {
  yield message;
}

// Async generator that yields no messages — used on retry to resume
// an existing session without sending a new user message.
async function* emptyMessages() {}

async function run() {
  // Track the latest session_id so we can resume on retry after rate limit.
  let currentSessionId = null;

  for (let attempt = 0; attempt <= rateLimitMaxRetries; attempt++) {
    let rateLimited = false;

    // On retry, resume the session captured from the previous attempt.
    const attemptOpts = { ...opts };
    let prompt;
    if (attempt === 0) {
      prompt = messages();
    } else {
      attemptOpts.resume = currentSessionId;
      prompt = emptyMessages();
    }

    const stream = query({ prompt, options: attemptOpts });

    // Wait for all MCP servers to be connected.
    const ourServers = new Set(Object.keys(attemptOpts.mcpServers || {}));
    if (ourServers.size > 0) {
      let first = true;
      let delay = 1;
      while (true) {
        const statuses = await stream.mcpServerStatus();
        if (first) {
          const statusNames = new Set(statuses.map((s) => s.name));
          const missing = [...ourServers].filter((n) => !statusNames.has(n));
          if (missing.length > 0) {
            throw new Error(
              "MCP servers not found in status list: " +
                missing.join(", ") +
                ". Available: " +
                [...statusNames].join(", ")
            );
          }
          first = false;
        }
        let pending = false;
        for (const s of statuses) {
          if (!ourServers.has(s.name)) continue;
          if (s.status === "failed" || s.status === "needs-auth") {
            throw new Error(
              "MCP server " +
                s.name +
                ": " +
                s.status +
                (s.error ? " - " + s.error : "")
            );
          }
          if (s.status === "pending") {
            pending = true;
          }
        }
        if (!pending) break;
        await new Promise((r) => setTimeout(r, delay));
        if (delay < 100) delay *= 2;
      }
    }

    // Stream events as JSONL to stdout.
    for await (const event of stream) {
      // Track session_id so we can resume on rate limit retry.
      if (event.session_id) {
        currentSessionId = event.session_id;
      }

      // Handle rate limit events
      if (
        event.type === "rate_limit_event" &&
        event.rate_limit_info?.status === "rejected"
      ) {
        const resetsAt = event.rate_limit_info.resetsAt;
        if (resetsAt && attempt < rateLimitMaxRetries) {
          const wait = Math.max(0, resetsAt - Date.now() / 1000) + 1;
          process.stderr.write(
            `Rate limited, retrying in ${Math.round(wait)}s (attempt ${attempt + 1}/${rateLimitMaxRetries})\n`
          );
          await new Promise((r) => setTimeout(r, wait * 1000));
          rateLimited = true;
          break;
        }
      }

      process.stdout.write(JSON.stringify(event) + "\n");
    }

    if (!rateLimited) break;
  }
}

run().catch((e) => {
  process.stderr.write(e.message || String(e));
  process.exit(1);
});
