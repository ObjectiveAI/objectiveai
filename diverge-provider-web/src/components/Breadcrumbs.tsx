import type { ReactNode } from "react";

export interface Crumb {
  url: string;
  label: string;
}

/**
 * The trail at the top of every page: where this page sits, ending in
 * the page itself. Shared by the front page and every section, so the
 * shape is decided once.
 */
export function Breadcrumbs(props: {
  crumbs: Crumb[];
  current: string;
}): ReactNode {
  return (
    <header>
      <nav aria-label="Breadcrumbs">
        <ol className="crumbs">
          {props.crumbs.map((crumb) => (
            <li key={crumb.url}>
              <a href={crumb.url}>{crumb.label}</a>
            </li>
          ))}
          <li aria-current="page">{props.current}</li>
        </ol>
      </nav>
    </header>
  );
}
