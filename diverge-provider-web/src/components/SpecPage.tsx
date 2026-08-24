import type { ReactNode } from "react";

import { Adjacent, type Neighbor } from "./Adjacent";
import { Breadcrumbs, type Crumb } from "./Breadcrumbs";
import { MarkdownLink } from "./MarkdownLink";
import { SpecNav, type NavLayer } from "./SpecNav";

/**
 * One specification section: breadcrumbs, the article, the outline,
 * and where to go next — the trail and the footer nav being the shared
 * [`Breadcrumbs`] and [`Adjacent`] the front page composes too.
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
      <Breadcrumbs crumbs={props.crumbs} current={props.title} />
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
            <MarkdownLink url={props.markdownUrl} />
          </article>
          <Adjacent previous={props.previous} next={props.next} />
        </main>
      </div>
    </div>
  );
}
