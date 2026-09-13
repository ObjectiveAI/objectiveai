import type { ReactNode } from "react";

export interface NavNode {
  url: string;
  title: string;
  children: NavNode[];
}

/**
 * Whether `node` is the current page or one of its ancestors. Every
 * URL ends in `/`, so a prefix match is exactly that and never a
 * sibling whose name happens to begin the same way.
 */
function onPath(node: NavNode, current: string): boolean {
  return current.startsWith(node.url);
}

function Link(props: { node: NavNode; current: string }): ReactNode {
  return (
    <a
      href={props.node.url}
      aria-current={props.node.url === props.current ? "page" : undefined}
    >
      {props.node.title}
    </a>
  );
}

function List(props: { nodes: NavNode[]; current: string }): ReactNode {
  return (
    <ol>
      {props.nodes.map((node) =>
        node.children.length > 0 ? (
          <li key={node.url}>
            <details open={onPath(node, props.current)}>
              <summary>
                <Link node={node} current={props.current} />
              </summary>
              <List nodes={node.children} current={props.current} />
            </details>
          </li>
        ) : (
          <li key={node.url}>
            <Link node={node} current={props.current} />
          </li>
        ),
      )}
    </ol>
  );
}

/**
 * The whole specification's outline, to its full depth, on every page.
 *
 * Every page linking every section is deliberate: internal linking is
 * how a crawler discovers the tree in one fetch, and how a reader
 * keeps their place in a nested document. Plain nested lists in a
 * `nav` landmark, and nothing runs: a section with children is a
 * native `details` whose `open` attribute the build decides for the
 * page — open for the current page and every section above it, so
 * the current section's children are listed, and closed everywhere
 * else — and which the reader toggles without a script.
 */
export function SpecNav(props: {
  nodes: NavNode[];
  current: string;
}): ReactNode {
  return (
    <nav aria-label="Specification contents">
      <List nodes={props.nodes} current={props.current} />
    </nav>
  );
}
