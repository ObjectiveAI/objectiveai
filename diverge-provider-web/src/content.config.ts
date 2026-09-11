import { glob } from "astro/loaders";
import { defineCollection, z } from "astro:content";

// The specification's sections. The directory tree IS the spec's
// outline: the first path segment is the REVISION — one module per
// revision, `src/content/spec/<version>/` — the second the layer, the
// third the section (a layer's own page is its `index`). Order comes
// from frontmatter rather than from filename prefixes, so the slugs
// stay clean.
const spec = defineCollection({
  // Ids keep the file path as written: the loader's default slugs
  // each segment and drops the dots out of `2.3.0`, and the version
  // is the first segment of every id.
  loader: glob({
    pattern: "**/*.mdx",
    base: "./src/content/spec",
    generateId: ({ entry }) => entry.replace(/\\/g, "/").replace(/\.mdx$/, ""),
  }),
  schema: z.object({
    /** The section's title, and its page's <h1>. */
    title: z.string(),
    /**
     * One sentence on what the section covers. It becomes the meta
     * description, the llms.txt line, and the stub's opening — one
     * string, three jobs, no drift.
     */
    summary: z.string(),
    /** Position among siblings: layers against layers, sections within a layer. */
    order: z.number(),
    /**
     * Whether the section's prose has actually been written. A stub
     * renders its summary and says plainly that the crate is the
     * source until the prose exists.
     */
    draft: z.boolean().default(true),
  }),
});

export const collections = { spec };
