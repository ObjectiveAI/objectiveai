import type { APIRoute } from "astro";

import { expandIncludes, expandLinks } from "../../include.mjs";
import { absolute, overview, versions } from "../../spec";

// A revision's overview twin, at the revision root's own path. The
// prose is the same collection entry the revision's front page
// renders — single-sourced — and only the header lines are computed,
// here, where computation belongs.
export async function getStaticPaths() {
  const paths = [];
  for (const version of await versions()) {
    paths.push({ params: { version }, props: { version } });
  }
  return paths;
}

export const GET: APIRoute<{ version: string }> = async ({ props }) => {
  const { version } = props;
  const page = await overview(version);
  const body = expandLinks(expandIncludes((page.entry.body ?? "").trim(), version), version);
  const text = [
    `# Diverge Provider Protocol — Specification ${version}`,
    "",
    `> ${page.entry.data.summary}`,
    "",
    `Canonical: ${absolute(page.url)}`,
    `Specification revision: ${version}`,
    "",
    body,
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
