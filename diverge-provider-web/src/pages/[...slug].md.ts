import type { APIRoute } from "astro";

import { expandIncludes } from "../include.mjs";
import { REVISION } from "../revision";
import { absolute, ordered, type Section } from "../spec";

// Every rendered page has a raw-markdown twin at the same path with a
// `.md` extension — one clean fetch per section for anything that
// would rather not parse HTML. The HTML page advertises it with
// `<link rel="alternate" type="text/markdown">`.
export async function getStaticPaths() {
  const all = await ordered();
  return all.map((section) => ({
    params: { slug: section.segments.join("/") },
    props: { section },
  }));
}

export const GET: APIRoute<{ section: Section }> = ({ props }) => {
  const { entry, url } = props.section;
  const body = expandIncludes((entry.body ?? "").trim());
  const text = [
    `# ${entry.data.title}`,
    "",
    `> ${entry.data.summary}`,
    "",
    `Canonical: ${absolute(url)}`,
    `Specification revision: ${REVISION}`,
    "",
    body,
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
