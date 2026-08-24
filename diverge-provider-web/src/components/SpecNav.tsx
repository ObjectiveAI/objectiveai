import type { ReactNode } from "react";

export interface NavNode {
  url: string;
  title: string;
  children: NavNode[];
}

function List(props: { nodes: NavNode[]; current: string }): ReactNode {
  return (
    <ol>
      {props.nodes.map((node) => (
        <li key={node.url}>
          <a
            href={node.url}
            aria-current={node.url === props.current ? "page" : undefined}
          >
            {node.title}
          </a>
          {node.children.length > 0 && (
            <List nodes={node.children} current={props.current} />
          )}
        </li>
      ))}
    </ol>
  );
}

/**
 * The whole specification's outline, to its full depth, on every page.
 *
 * Every page linking every section is deliberate: internal linking is
 * how a crawler discovers the tree in one fetch, and how a reader
 * keeps their place in a nested document. Plain nested lists — a `nav`
 * landmark, no state, nothing collapsed, because nothing here runs.
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
