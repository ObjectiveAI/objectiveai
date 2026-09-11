import type { ReactNode } from "react";

export interface Neighbor {
  url: string;
  title: string;
}

/**
 * The footer nav on every page: the section before on the left, the
 * section after on the right, either absent at the ends. Shared by the
 * front page and every section, so the shape is decided once.
 */
export function Adjacent(props: {
  previous: Neighbor | null;
  next: Neighbor | null;
}): ReactNode {
  return (
    <nav aria-label="Adjacent sections" className="adjacent">
      {props.previous ? (
        <a rel="prev" href={props.previous.url}>
          ← {props.previous.title}
        </a>
      ) : (
        <span />
      )}
      {props.next ? (
        <a rel="next" href={props.next.url}>
          {props.next.title} →
        </a>
      ) : (
        <span />
      )}
    </nav>
  );
}
