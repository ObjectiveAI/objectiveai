import type { ReactNode } from "react";

import { SpecNav, type NavLayer } from "./SpecNav";

export interface Crumb {
  url: string;
  label: string;
}

export interface Neighbor {
  url: string;
  title: string;
}

/**
 * One specification page: breadcrumbs, the article, the outline, and
 * where to go next.
 *
 * React as an authoring language and nothing else — this renders once,
 * at build time, and ships as HTML. The article's body arrives as
 * `children`, already rendered from the section's markdown.
 */
export function SpecPage(props: {
  title: string;
  summary: string;
  markdownUrl: string;
  crumbs: Crumb[];
  layers: NavLayer[];
  current: string;
  previous: Neighbor | null;
  next: Neighbor | null;
  draft: boolean;
  children: ReactNode;
}): ReactNode {
  return (
    <div className="page">
      <header>
        <nav aria-label="Breadcrumbs">
          <ol className="crumbs">
            {props.crumbs.map((crumb) => (
              <li key={crumb.url}>
                <a href={crumb.url}>{crumb.label}</a>
              </li>
            ))}
            <li aria-current="page">{props.title}</li>
          </ol>
        </nav>
      </header>
      <div className="columns">
        <aside>
          <SpecNav layers={props.layers} current={props.current} />
        </aside>
        <main>
          <article>
            <h1>{props.title}</h1>
            <p className="summary">{props.summary}</p>
            {props.draft && (
              <p className="draft" role="note">
                This section is not yet written.
              </p>
            )}
            {props.children}
            <footer>
              <p>
                <a href={props.markdownUrl} type="text/markdown">
                  This page as Markdown
                </a>
              </p>
            </footer>
          </article>
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
        </main>
      </div>
    </div>
  );
}
