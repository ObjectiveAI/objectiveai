import { useEffect, useRef } from "react";

/** Roving focus along a row (the tab strip): where ArrowLeft/ArrowRight, Home or End moves from `at`,
 *  round the ends. Any other key moves nowhere (null). */
export function rove(count: number, at: number, key: string): number | null {
  if (count <= 0) return null;
  const here = Math.min(Math.max(at, 0), count - 1);
  switch (key) {
    case "ArrowRight":
      return (here + 1) % count;
    case "ArrowLeft":
      return (here + count - 1) % count;
    case "Home":
      return 0;
    case "End":
      return count - 1;
    default:
      return null;
  }
}

/** Whether a key opens what has focus, as a press would. */
export const opens = (key: string) => key === "Enter" || key === " ";

/** Forms open inline, newest last. Escape closes the newest one still on screen (not in a hidden tab). */
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
