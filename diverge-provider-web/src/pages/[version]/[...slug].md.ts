import type { APIRoute } from "astro";

import { expandIncludes, expandLinks } from "../../include.mjs";
import { absolute, ordered, versions, type Section } from "../../spec";

// Every rendered page has a raw-markdown twin at the same path with a
// `.md` extension — one clean fetch per section for anything that
// would rather not parse HTML. The HTML page advertises it with
// `<link rel="alternate" type="text/markdown">`.
export async function getStaticPaths() {
  const paths = [];
  for (const version of await versions()) {
    for (const section of await ordered(version)) {
      paths.push({
        params: { version, slug: section.segments.join("/") },
        props: { section },
      });
    }
  }
  return paths;
}

export const GET: APIRoute<{ section: Section }> = ({ props }) => {
  const { entry, url, version } = props.section;
  const body = expandLinks(expandIncludes((entry.body ?? "").trim(), version), version);
  const text = [
    `# ${entry.data.title}`,
    "",
    `> ${entry.data.summary}`,
    "",
    `Canonical: ${absolute(url)}`,
    `Specification revision: ${version}`,
    "",
    body,
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
