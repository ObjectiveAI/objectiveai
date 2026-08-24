import type { ReactNode } from "react";

export interface NavSection {
  url: string;
  title: string;
}

export interface NavLayer {
  index: NavSection;
  sections: NavSection[];
}

/**
 * The whole specification's outline, on every page.
 *
 * Every page linking every section is deliberate: internal linking is
 * how a crawler discovers the tree in one fetch, and how a reader
 * keeps their place in a layered document. Plain nested lists — a
 * `nav` landmark, no state, nothing collapsed, because nothing here
 * runs.
 */
export function SpecNav(props: {
  layers: NavLayer[];
  current: string;
}): ReactNode {
  return (
    <nav aria-label="Specification contents">
      <ol>
        {props.layers.map((layer) => (
          <li key={layer.index.url}>
            <a
              href={layer.index.url}
              aria-current={
                layer.index.url === props.current ? "page" : undefined
              }
            >
              {layer.index.title}
            </a>
            {layer.sections.length > 0 && (
              <ol>
                {layer.sections.map((section) => (
                  <li key={section.url}>
                    <a
                      href={section.url}
                      aria-current={
                        section.url === props.current ? "page" : undefined
                      }
                    >
                      {section.title}
                    </a>
                  </li>
                ))}
              </ol>
            )}
          </li>
        ))}
      </ol>
    </nav>
  );
}
