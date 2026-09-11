import type { APIRoute } from "astro";

import { expandIncludes } from "../include.mjs";
import { REVISION } from "../revision";
import { absolute, overview } from "../spec";

// The Overview's raw-Markdown twin, at the root's own path. The prose
// is the same collection entry the front page renders — single-sourced
// — and only the header lines are computed, here, where computation
// belongs.
export const GET: APIRoute = async () => {
  const page = await overview();
  const body = expandIncludes((page.entry.body ?? "").trim());
  const text = [
    `# Diverge Provider Protocol — Specification ${REVISION}`,
    "",
    `> ${page.entry.data.summary}`,
    "",
    `Canonical: ${absolute(page.url)}`,
    `Specification revision: ${REVISION}`,
    "",
    body,
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
