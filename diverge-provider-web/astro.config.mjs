// @ts-check
import mdx from "@astrojs/mdx";
import react from "@astrojs/react";
import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";
import rehypeAutolinkHeadings from "rehype-autolink-headings";
import rehypeSlug from "rehype-slug";

import { remarkInclude, remarkVersionLinks } from "./src/include.mjs";

// The Diverge Provider Protocol specification.
//
// Everything renders to static HTML at build time and NOTHING ships
// JavaScript: React is an authoring language here, not a runtime. No
// component carries a `client:*` directive, and the build is checked
// for the absence of <script> tags.
export default defineConfig({
  site: "https://protocol.diverge.network",
  // One canonical URL per page. Crawlers punish duplicates, and a
  // trailing-slash policy chosen once means no 301 chains and no
  // split page identity.
  trailingSlash: "always",
  integrations: [react(), mdx(), sitemap()],
  markdown: {
    // Shiki highlights at build time into inline-styled spans — the
    // zero-JS gate stays true, and the raw .md twins keep plain fenced
    // blocks. One theme, since the site is dark-only; its token colors
    // are the mark's family (gold, rose, iris on lavender), and Layout
    // paints the block's background from the page's own palette.
    shikiConfig: {
      theme: "rose-pine-moon",
    },
    // A fenced block whose meta names a crate file is that file, read
    // at build time, and a module's root-relative links are carried
    // into its version — see `src/include.mjs`. Before Shiki, so what
    // is highlighted is the file.
    remarkPlugins: [remarkInclude, remarkVersionLinks],
    rehypePlugins: [
      // Every heading gets a stable id, and a visible anchor link:
      // deep links are half of what makes a specification citable.
      rehypeSlug,
      [
        rehypeAutolinkHeadings,
        {
          behavior: "append",
          properties: { className: ["anchor"], ariaLabel: "Link to this section" },
          content: { type: "text", value: "#" },
        },
      ],
    ],
  },
});
