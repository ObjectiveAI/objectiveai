import { useEffect, useRef } from "react";

/** Forms open inline, newest last. Escape closes the newest one still on screen (not in a hidden page). */
export type Escapable = { close: () => void; shown: () => boolean };

export function escapeTop(open: Escapable[]): Escapable | null {
  for (let i = open.length - 1; i >= 0; i--) if (open[i].shown()) return open[i];
  return null;
}

const OPEN: Escapable[] = [];
let listening = false;

function onKey(e: KeyboardEvent) {
  if (e.key !== "Escape" || e.defaultPrevented) return;
  const top = escapeTop(OPEN);
  if (!top) return;
  e.preventDefault();
  top.close();
}

/** While `open`, Escape closes this inline form (or confirm). Put the returned ref on the form's own element:
 *  a form in a tab you're not on is left alone. */
export function useEscape<T extends HTMLElement>(open: boolean, close: () => void) {
  const ref = useRef<T>(null);
  const closeRef = useRef(close);
  closeRef.current = close;
  useEffect(() => {
    if (!open) return;
    const entry: Escapable = {
      close: () => closeRef.current(),
      shown: () => {
        const el = ref.current;
        return !!el && el.isConnected && !el.closest("[hidden]");
      },
    };
    OPEN.push(entry);
    if (!listening) {
      window.addEventListener("keydown", onKey);
      listening = true;
    }
    return () => {
      const i = OPEN.indexOf(entry);
      if (i >= 0) OPEN.splice(i, 1);
    };
  }, [open]);
  return ref;
}
