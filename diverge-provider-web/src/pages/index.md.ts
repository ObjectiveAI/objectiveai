import type { APIRoute } from "astro";

import { REVISION } from "../revision";
import { absolute } from "../spec";

// The Overview's raw-Markdown twin, at the root's own path. The prose
// mirrors src/pages/index.astro — the front page is authored as JSX
// for its dynamic links, so its twin is maintained beside it; keep the
// two in step when either changes.
export const GET: APIRoute = () => {
  const text = [
    `# Diverge Provider Protocol — Specification ${REVISION}`,
    "",
    "> The wire protocol between callers and providers: running " +
      "agentic loops, MCP plugins, and laboratories; managing " +
      "volumes; checking images. One WebSocket connection, framed " +
      "and multiplexed, with a handshake in front.",
    "",
    `Canonical: ${absolute("/")}`,
    `Specification revision: ${REVISION}`,
    "",
    "This specification is the protocol's normative definition. " +
      "[`diverge-provider-sdk`](https://docs.rs/diverge-provider-sdk) " +
      "is a Rust SDK that implements it, versioned with this " +
      "specification's revisions.",
    "",
    "The whole specification is also available as " +
      `[a single page](${absolute(`/${REVISION}/full/`)}), as ` +
      `[one plain-text file](${absolute("/llms-full.txt")}), and ` +
      "section by section as raw Markdown — every page links its own " +
      `\`.md\` twin, and [/llms.txt](${absolute("/llms.txt")}) indexes ` +
      "them all. The crate and this site live in " +
      "[one repository](https://github.com/ObjectiveAI/objectiveai).",
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
