import { getCollection, type CollectionEntry } from "astro:content";

import { ORIGIN, REVISION } from "./revision";

/** One section of the specification, located. */
export interface Section {
  entry: CollectionEntry<"spec">;
  /** The revision the section belongs to: its module. */
  version: string;
  /** The section's path segments within its revision, however deep. */
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
  /** Ancestors, outermost first. The revision's root page is not among them. */
  trail: Section[];
  children: Section[];
}

/** The recursive shape the sidebar renders. */
export interface NavNode {
  url: string;
  title: string;
  children: NavNode[];
}

/** One entry of the version menu: where this page is in that revision. */
export interface Switch {
  version: string;
  url: string;
}

function locate(entry: CollectionEntry<"spec">): Section {
  const parts = entry.id.split("/");
  const version = parts[0];
  const segments = parts.slice(1).filter((part) => part !== "index");
  const path = [version, ...segments].join("/");
  return {
    entry,
    version,
    segments,
    url: `/${path}/`,
    markdownUrl: `/${path}.md`,
  };
}

async function sections(): Promise<Section[]> {
  const entries = await getCollection("spec");
  return entries.map(locate);
}

/** Semantic-version order, newest first. */
function compare(a: string, b: string): number {
  const pa = a.split(".").map((part) => Number.parseInt(part, 10));
  const pb = b.split(".").map((part) => Number.parseInt(part, 10));
  for (let i = 0; i < Math.max(pa.length, pb.length); i += 1) {
    const da = Number.isNaN(pa[i] ?? NaN) ? 0 : (pa[i] ?? 0);
    const db = Number.isNaN(pb[i] ?? NaN) ? 0 : (pb[i] ?? 0);
    if (da !== db) {
      return db - da;
    }
  }
  return b.localeCompare(a);
}

/**
 * Every revision the site holds — one content module each, under
 * `src/content/spec/<version>/` — newest first. The crate's version
 * is among them, or the build says so: the latest revision is the
 * crate's, and a crate that moved on without its module is a site
 * with nothing to show for it.
 */
export async function versions(): Promise<string[]> {
  const all = new Set((await sections()).map((section) => section.version));
  if (!all.has(REVISION)) {
    throw new Error(
      `the crate is at ${REVISION} and src/content/spec/${REVISION}/ does not exist`,
    );
  }
  return [...all].sort(compare);
}

/** The latest revision: the crate's version. */
export function latest(): string {
  return REVISION;
}

/**
 * One revision as a tree, however deep the sections nest. A node
 * with children is a directory with an `index.mdx` beside them; a leaf
 * is a file. Siblings order by frontmatter `order`. A child whose
 * parent has no index is an authoring error, and the build says so.
 *
 * The overview is not in the tree: it is the revision's root page,
 * fetched by [`overview`], and the navigation hardcodes its entry
 * first.
 */
export async function tree(version: string): Promise<Node[]> {
  const own = (await sections()).filter(
    (section) => section.version === version && section.segments[0] !== "overview",
  );
  const byPath = new Map<string, Node>();
  for (const section of own) {
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
        `spec section "${version}/${node.section.segments.join("/")}" has no parent index`,
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
 * Every section of one revision in reading order — the tree, depth
 * first — each with its ancestor trail and its children.
 */
export async function flattened(version: string): Promise<Located[]> {
  const roots = await tree(version);
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

/** Every section of one revision in reading order. */
export async function ordered(version: string): Promise<Section[]> {
  return (await flattened(version)).map((located) => located.section);
}

/**
 * The sidebar for one revision: Overview first, pointing at the
 * revision's root — the overview IS its front page — then the whole
 * tree, to its full depth, on every page.
 */
export async function navigation(version: string): Promise<NavNode[]> {
  const toNav = (node: Node): NavNode => ({
    url: node.section.url,
    title: node.section.entry.data.title,
    children: node.children.map(toNav),
  });
  return [
    { url: `/${version}/`, title: "Overview", children: [] },
    ...(await tree(version)).map(toNav),
  ];
}

/**
 * One revision's overview: its front page's prose, single-sourced for
 * the page and its twin. Its URLs are the revision root's own, not a
 * section's.
 */
export async function overview(version: string): Promise<Section> {
  const entry = (await sections()).find(
    (section) =>
      section.version === version &&
      section.segments.length === 1 &&
      section.segments[0] === "overview",
  );
  if (!entry) {
    throw new Error(`src/content/spec/${version}/overview/index.mdx is missing`);
  }
  return {
    entry: entry.entry,
    version,
    segments: [],
    url: `/${version}/`,
    markdownUrl: `/${version}/index.md`,
  };
}

/**
 * The version menu for one page: every revision, and where this page
 * is in each — the same segments when the revision has that section,
 * its root when it does not.
 */
export async function switches(segments: string[]): Promise<Switch[]> {
  const all = await sections();
  const path = segments.join("/");
  return (await versions()).map((version) => {
    const exists =
      segments.length === 0 ||
      all.some(
        (section) =>
          section.version === version && section.segments.join("/") === path,
      );
    return {
      version,
      url: exists ? `/${[version, ...segments].join("/")}/` : `/${version}/`,
    };
  });
}

/** An absolute URL for a site-relative path. */
export function absolute(path: string): string {
  return `${ORIGIN}${path}`;
}
