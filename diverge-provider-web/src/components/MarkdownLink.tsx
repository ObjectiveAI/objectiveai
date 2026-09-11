import type { ReactNode } from "react";

/**
 * The article footer on every page: the pointer to this page's
 * raw-Markdown twin. Shared by the front page and every section, so
 * the shape is decided once — and so no page can forget to advertise
 * the twin the crawler artifacts promise.
 */
export function MarkdownLink(props: { url: string }): ReactNode {
  return (
    <footer>
      <p>
        <a href={props.url} type="text/markdown">
          This page as Markdown
        </a>
      </p>
    </footer>
  );
}
