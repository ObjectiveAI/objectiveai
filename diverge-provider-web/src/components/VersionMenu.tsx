import type { ReactNode } from "react";

import type { Switch } from "../spec";

/**
 * The revision selector at the head of every page: a native
 * `<details>` — the summary names the revision this page belongs to,
 * and the list beneath it links every revision the site holds, to
 * this page where the revision has it and to the revision's root
 * where it does not. No script: the disclosure is the browser's own.
 */
export function VersionMenu(props: {
  current: string;
  switches: Switch[];
}): ReactNode {
  return (
    <details className="versions">
      <summary>Specification {props.current} ▾</summary>
      <ul>
        {props.switches.map((entry) => (
          <li key={entry.version}>
            <a
              href={entry.url}
              aria-current={entry.version === props.current ? "page" : undefined}
            >
              {entry.version}
            </a>
          </li>
        ))}
      </ul>
    </details>
  );
}
