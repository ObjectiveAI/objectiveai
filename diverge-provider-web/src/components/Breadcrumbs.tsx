import type { ReactNode } from "react";

import type { Switch } from "../spec";
import { VersionMenu } from "./VersionMenu";

export interface Crumb {
  url: string;
  label: string;
}

/** The revision this page belongs to, and where it is in every other. */
export interface Menu {
  current: string;
  switches: Switch[];
}

/**
 * The trail at the top of every page: the revision menu first, then
 * where this page sits, ending in the page itself. Shared by every
 * revision's front page and every section, so the shape is decided
 * once.
 */
export function Breadcrumbs(props: {
  menu: Menu;
  crumbs: Crumb[];
  current: string;
}): ReactNode {
  return (
    <header>
      <nav aria-label="Breadcrumbs">
        <ol className="crumbs">
          <li>
            <VersionMenu current={props.menu.current} switches={props.menu.switches} />
          </li>
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
