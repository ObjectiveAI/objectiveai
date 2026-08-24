import { getCollection, type CollectionEntry } from "astro:content";

import { ORIGIN } from "./revision";

/** One section of the specification, located. */
export interface Section {
  entry: CollectionEntry<"spec">;
  /** The section's path segments, however deep. */
  segments: string[];
  /** Site-relative URL of the rendered page, with trailing slash. */
  url: string;
  /** Site-relative URL of the raw-markdown twin. */
  markdownUrl: string;
}

/** A section and the sections beneath it. */
export interface Node {
  section: Section;
  children: Node[];
}

/** A section with its place: ancestors above, children below. */
export interface Located {
  section: Section;
  /** Ancestors, outermost first. The root page is not among them. */
  trail: Section[];
  children: Section[];
}

/** The recursive shape the sidebar renders. */
export interface NavNode {
  url: string;
  title: string;
  children: NavNode[];
}

function locate(entry: CollectionEntry<"spec">): Section {
  const segments = entry.id.split("/").filter((part) => part !== "index");
  const path = segments.join("/");
  return {
    entry,
    segments,
    url: `/${path}/`,
    markdownUrl: `/${path}.md`,
  };
}

/**
 * The specification as a tree, however deep the sections nest. A node
 * with children is a directory with an `index.mdx` beside them; a leaf
 * is a file. Siblings order by frontmatter `order`. A child whose
 * parent has no index is an authoring error, and the build says so.
 *
 * The overview is not in the tree: it is the root page, fetched by
 * [`overview`], and the navigation hardcodes its entry first.
 */
export async function tree(): Promise<Node[]> {
  const entries = await getCollection("spec");
  const sections = entries
    .map(locate)
    .filter((section) => section.segments[0] !== "overview");
  const byPath = new Map<string, Node>();
  for (const section of sections) {
    byPath.set(section.segments.join("/"), { section, children: [] });
  }
  const roots: Node[] = [];
  for (const node of byPath.values()) {
    const parentPath = node.section.segments.slice(0, -1).join("/");
    if (parentPath === "") {
      roots.push(node);
      continue;
    }
    const parent = byPath.get(parentPath);
    if (!parent) {
      throw new Error(
        `spec section "${node.section.segments.join("/")}" has no parent index`,
      );
    }
    parent.children.push(node);
  }
  const sort = (nodes: Node[]) => {
    nodes.sort(
      (a, b) => a.section.entry.data.order - b.section.entry.data.order,
    );
    for (const node of nodes) {
      sort(node.children);
    }
  };
  sort(roots);
  return roots;
}

/**
 * Every section in reading order — the tree, depth first — each with
 * its ancestor trail and its children.
 */
export async function flattened(): Promise<Located[]> {
  const roots = await tree();
  const out: Located[] = [];
  const walk = (node: Node, trail: Section[]) => {
    out.push({
      section: node.section,
      trail,
      children: node.children.map((child) => child.section),
    });
    for (const child of node.children) {
      walk(child, [...trail, node.section]);
    }
  };
  for (const root of roots) {
    walk(root, []);
  }
  return out;
}

/** Every section in reading order. */
export async function ordered(): Promise<Section[]> {
  return (await flattened()).map((located) => located.section);
}

/**
 * The sidebar: Overview first, pointing at the root — the overview IS
 * the front page — then the whole tree, to its full depth, on every
 * page.
 */
export async function navigation(): Promise<NavNode[]> {
  const toNav = (node: Node): NavNode => ({
    url: node.section.url,
    title: node.section.entry.data.title,
    children: node.children.map(toNav),
  });
  return [
    { url: "/", title: "Overview", children: [] },
    ...(await tree()).map(toNav),
  ];
}

/**
 * The overview: the front page's prose, single-sourced for the page
 * and its twin. Its URLs are the root's own, not a section's.
 */
export async function overview(): Promise<Section> {
  const entries = await getCollection("spec");
  // The glob loader names a directory's index by the directory alone.
  const entry = entries.find(
    (candidate) =>
      candidate.id === "overview" || candidate.id === "overview/index",
  );
  if (!entry) {
    throw new Error("src/content/spec/overview/index.mdx is missing");
  }
  return {
    entry,
    segments: [],
    url: "/",
    markdownUrl: "/index.md",
  };
}

/** An absolute URL for a site-relative path. */
export function absolute(path: string): string {
  return `${ORIGIN}${path}`;
}
