import type { APIRoute } from "astro";

import { REVISION } from "../../revision";
import { absolute, ordered, type Section } from "../../spec";

// Every rendered page has a raw-markdown twin at the same path with a
// `.md` extension — one clean fetch per section for anything that
// would rather not parse HTML. The HTML page advertises it with
// `<link rel="alternate" type="text/markdown">`, and llms.txt indexes
// these rather than the HTML.
export async function getStaticPaths() {
  const all = await ordered();
  return all.map((section) => ({
    params: {
      revision: REVISION,
      slug:
        section.section === null
          ? section.layer
          : `${section.layer}/${section.section}`,
    },
    props: { section },
  }));
}

export const GET: APIRoute<{ section: Section }> = ({ props }) => {
  const { entry, url } = props.section;
  const body = (entry.body ?? "").trim();
  const text = [
    `# ${entry.data.title}`,
    "",
    `> ${entry.data.summary}`,
    "",
    `Canonical: ${absolute(url)}`,
    `Specification revision: ${REVISION} (the version of the normative`,
    "`diverge-provider-sdk` crate).",
    "",
    body,
    "",
  ].join("\n");
  return new Response(text, {
    headers: { "Content-Type": "text/markdown; charset=utf-8" },
  });
};
