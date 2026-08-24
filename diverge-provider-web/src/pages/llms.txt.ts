import type { APIRoute } from "astro";

import { REVISION } from "../revision";
import { absolute, layers } from "../spec";

// The llms.txt convention (llmstxt.org): a Markdown index built for
// language-model consumption. Every link points at a section's raw
// `.md` twin rather than its HTML, so one fetch per section yields
// clean text.
export const GET: APIRoute = async () => {
  const tree = await layers();
  const lines: string[] = [
    "# Diverge Provider Protocol",
    "",
    "> The specification of the Diverge Provider Protocol: the wire " +
      "protocol between callers and providers for running agentic " +
      "loops, MCP plugins, and laboratories, and for managing volumes " +
      "and images.",
    "",
    `The current revision is ${REVISION}, which is the version of the ` +
      "normative `diverge-provider-sdk` Rust crate. The crate defines " +
      "the protocol's messages; where prose and crate disagree, the " +
      "crate is correct.",
    "",
    "Every specification page has a raw-Markdown twin at the same URL " +
      "with a `.md` extension; those twins are what this file links. " +
      "The entire specification concatenated is /llms-full.txt.",
    "",
    "## Specification",
    "",
  ];
  for (const layer of tree) {
    lines.push(
      `- [${layer.index.entry.data.title}](${absolute(layer.index.markdownUrl)}): ${layer.index.entry.data.summary}`,
    );
    for (const section of layer.sections) {
      lines.push(
        `- [${section.entry.data.title}](${absolute(section.markdownUrl)}): ${section.entry.data.summary}`,
      );
    }
  }
  lines.push(
    "",
    "## Optional",
    "",
    `- [Complete specification, one file](${absolute("/llms-full.txt")}): every section above, concatenated in reading order`,
    `- [Rendered specification](${absolute(`/specification/${REVISION}/`)}): the HTML table of contents`,
    "- [diverge-provider-sdk on docs.rs](https://docs.rs/diverge-provider-sdk): the normative crate's API documentation",
    "",
  );
  return new Response(lines.join("\n"), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
};
