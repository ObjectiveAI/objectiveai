import type { APIRoute } from "astro";

import { REVISION } from "../revision";
import { absolute, ordered } from "../spec";

// The whole specification in one fetch: every section's Markdown,
// concatenated in reading order. The companion of /llms.txt, for
// consumers that want the document rather than the index.
export const GET: APIRoute = async () => {
  const all = await ordered();
  const parts: string[] = [
    "# Diverge Provider Protocol — Specification " + REVISION,
    "",
    "> The complete specification, concatenated in reading order. The " +
      "revision is the version of the normative `diverge-provider-sdk` " +
      "Rust crate; where prose and crate disagree, the crate is correct.",
    "",
  ];
  for (const section of all) {
    parts.push(
      "---",
      "",
      `# ${section.entry.data.title}`,
      "",
      `> ${section.entry.data.summary}`,
      "",
      `Canonical: ${absolute(section.url)}`,
      "",
      (section.entry.body ?? "").trim(),
      "",
    );
  }
  return new Response(parts.join("\n"), {
    headers: { "Content-Type": "text/plain; charset=utf-8" },
  });
};
