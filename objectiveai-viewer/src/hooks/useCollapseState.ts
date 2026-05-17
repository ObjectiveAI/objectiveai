import { useState, useCallback, useRef, useEffect } from "react";
import type { Entry } from "../types";
import { getEntrySummary } from "../lib/entrySummary";

export function useCollapseState(entries: Entry[]) {
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const userExpanded = useRef<Set<string>>(new Set());
  const prevLength = useRef(0);

  useEffect(() => {
    if (entries.length <= 10 || entries.length <= prevLength.current) {
      prevLength.current = entries.length;
      return;
    }
    prevLength.current = entries.length;

    setCollapsed((prev) => {
      const next = new Set(prev);
      const lastId = entries[entries.length - 1]?.id;
      for (const e of entries) {
        if (e.id === lastId) continue;
        if (userExpanded.current.has(e.id)) continue;
        if (next.has(e.id)) continue;
        const summary = getEntrySummary(e);
        if (summary.status === "complete") next.add(e.id);
      }
      return next.size === prev.size ? prev : next;
    });
  }, [entries]);

  const toggle = useCallback((id: string) => {
    setCollapsed((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
        userExpanded.current.add(id);
      } else {
        next.add(id);
        userExpanded.current.delete(id);
      }
      return next;
    });
  }, []);

  const collapseAll = useCallback(() => {
    userExpanded.current.clear();
    setCollapsed(new Set(entries.map((e) => e.id)));
  }, [entries]);

  const expandAll = useCallback(() => {
    userExpanded.current = new Set(entries.map((e) => e.id));
    setCollapsed(new Set());
  }, [entries]);

  const isCollapsed = useCallback((id: string) => collapsed.has(id), [collapsed]);

  return { collapsed, toggle, collapseAll, expandAll, isCollapsed };
}
