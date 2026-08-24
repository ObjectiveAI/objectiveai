import { getCollection, type CollectionEntry } from "astro:content";

import { ORIGIN, REVISION } from "./revision";

/** One section of the specification, located. */
export interface Section {
  entry: CollectionEntry<"spec">;
  /** The layer's path segment. */
  layer: string;
  /** The section's path segment, or null for the layer's own page. */
  section: string | null;
  /** Site-relative URL of the rendered page, with trailing slash. */
  url: string;
  /** Site-relative URL of the raw-markdown twin. */
  markdownUrl: string;
}

/** One layer: its own page first, then its sections in order. */
export interface Layer {
  index: Section;
  sections: Section[];
}

function locate(entry: CollectionEntry<"spec">): Section {
  const parts = entry.id.split("/");
  const layer = parts[0];
  const section =
    parts.length > 1 && parts[1] !== "index" ? parts[1] : null;
  const path = section === null ? layer : `${layer}/${section}`;
  return {
    entry,
    layer,
    section,
    url: `/${REVISION}/${path}/`,
    markdownUrl: `/${REVISION}/${path}.md`,
  };
}

/**
 * The whole specification, in reading order: layers by their index
 * page's `order`, sections by their own within each layer.
 */
export async function layers(): Promise<Layer[]> {
  const entries = await getCollection("spec");
  const located = entries.map(locate);
  const indexes = located
    .filter((s) => s.section === null)
    .sort((a, b) => a.entry.data.order - b.entry.data.order);
  return indexes.map((index) => ({
    index,
    sections: located
      .filter((s) => s.layer === index.layer && s.section !== null)
      .sort((a, b) => a.entry.data.order - b.entry.data.order),
  }));
}

/**
 * The sidebar: Overview first, pointing at the root — the overview IS
 * the front page — then every layer and section from the collection.
 */
export async function navigation(): Promise<
  { index: { url: string; title: string }; sections: { url: string; title: string }[] }[]
> {
  const tree = await layers();
  return [
    { index: { url: "/", title: "Overview" }, sections: [] },
    ...tree.map((layer) => ({
      index: { url: layer.index.url, title: layer.index.entry.data.title },
      sections: layer.sections.map((child) => ({
        url: child.url,
        title: child.entry.data.title,
      })),
    })),
  ];
}

/** Every section in reading order, layer pages included. */
export async function ordered(): Promise<Section[]> {
  return (await layers()).flatMap((layer) => [
    layer.index,
    ...layer.sections,
  ]);
}

/** An absolute URL for a site-relative path. */
export function absolute(path: string): string {
  return `${ORIGIN}${path}`;
}
