import type { ReactNode } from "react";

import { Adjacent, type Neighbor } from "./Adjacent";
import { Breadcrumbs, type Crumb, type Menu } from "./Breadcrumbs";
import { MarkdownLink } from "./MarkdownLink";
import { SpecNav, type NavNode } from "./SpecNav";

export interface Subsection {
  url: string;
  title: string;
  summary: string;
}

/**
 * One specification section: breadcrumbs, the article, the outline,
 * and where to go next — the trail and the footer nav being the shared
 * [`Breadcrumbs`] and [`Adjacent`] the front page composes too.
 *
 * A section with children enumerates them after its prose: the listing
 * is how a reader descends and how a crawler discovers the subtree
 * from the page itself.
 *
 * React as an authoring language and nothing else — this renders once,
 * at build time, and ships as HTML. The article's body arrives as
 * `children`, already rendered from the section's markdown.
 */
export function SpecPage(props: {
  title: string;
  summary: string;
  markdownUrl: string;
  menu: Menu;
  crumbs: Crumb[];
  nodes: NavNode[];
  current: string;
  previous: Neighbor | null;
  next: Neighbor | null;
  draft: boolean;
  subsections: Subsection[];
  children: ReactNode;
}): ReactNode {
  return (
    <div className="page">
      <Breadcrumbs menu={props.menu} crumbs={props.crumbs} current={props.title} />
      <div className="columns">
        <aside>
          <SpecNav nodes={props.nodes} current={props.current} />
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
            {props.subsections.length > 0 && (
              <nav aria-label="Subsections">
                <h2 id="subsections">In this section</h2>
                <ul>
                  {props.subsections.map((subsection) => (
                    <li key={subsection.url}>
                      <a href={subsection.url}>{subsection.title}</a> —{" "}
                      {subsection.summary}
                    </li>
                  ))}
                </ul>
              </nav>
            )}
            <MarkdownLink url={props.markdownUrl} />
          </article>
          <Adjacent previous={props.previous} next={props.next} />
        </main>
      </div>
    </div>
  );
}
