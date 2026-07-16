import { memo, useMemo, useState, type ReactElement } from "react";
import cn from "classnames";
import type { ViewerTransport } from "@objectiveai/sdk";
import {
  useLaboratoryFiletree,
  type FileTreeNode,
} from "../hooks/useLaboratoryFiletree";
import { formatBytes, formatTimestamp } from "../lib/format";
import { LogoMark } from "./shared/Logo";

/**
 * The laboratory filesystem browser — a VSCode-style expandable tree
 * with details columns (name, size, created, modified, created by,
 * modified by). Browse only; no file opening yet.
 *
 * RE-RENDERS ARE REGION-LOCAL by construction: the filetree fold is
 * path-copying immutable (only the changed spine gets new object
 * identities), each tree node is a `memo`ized component keyed on its
 * node object, and expansion state lives INSIDE each node component —
 * so a filesystem event re-renders exactly the root→changed-node
 * spine, and expanding a directory re-renders that subtree alone.
 * Don't introduce a shared expansion Set or a flat row model: both
 * would couple every row's render to every change.
 */

/** The one shared column template — every row AND the header use it,
 * so columns align across depths without a <table>. */
const GRID_COLS =
  "grid-cols-[minmax(0,1fr)_6rem_9.5rem_9.5rem_12rem_12rem]";

/** The sortable columns, in header order. */
const SORT_COLUMNS = [
  "name",
  "size",
  "created",
  "modified",
  "created by",
  "modified by",
] as const;
type SortColumn = (typeof SORT_COLUMNS)[number];

/** The active sort: one column, one direction — or none (the default
 * name order). Cycled per column by header clicks: asc → desc → off. */
type Sort = { column: SortColumn; descending: boolean } | null;

/** A node's value under a sort column; `null` sorts LAST in both
 * directions (missing metadata never floats to the top). */
function sortKey(node: FileTreeNode, column: SortColumn): string | number | null {
  switch (column) {
    case "name":
      return node.name;
    case "size":
      return node.type === "file" ? (node.size ?? null) : null;
    case "created":
      return node.created_at ?? null;
    case "modified":
      return node.modified_at ?? null;
    case "created by":
      return node.created_by ?? null;
    case "modified by":
      return node.modified_by ?? null;
  }
}

/** Directories first always; within each group the active sort (name
 * order when none), applied at EVERY level of the tree. */
function sortEntries(nodes: FileTreeNode[], sort: Sort): FileTreeNode[] {
  return [...nodes].sort((a, b) => {
    const group =
      (a.type === "directory" ? 0 : 1) - (b.type === "directory" ? 0 : 1);
    if (group !== 0) {
      return group;
    }
    if (sort !== null) {
      const ka = sortKey(a, sort.column);
      const kb = sortKey(b, sort.column);
      if (ka !== kb) {
        if (ka === null) {
          return 1;
        }
        if (kb === null) {
          return -1;
        }
        const cmp =
          typeof ka === "number" && typeof kb === "number"
            ? ka - kb
            : String(ka).localeCompare(String(kb));
        if (cmp !== 0) {
          return sort.descending ? -cmp : cmp;
        }
      }
    }
    return a.name.localeCompare(b.name);
  });
}

/** A metadata cell: the value, or a dim `—` placeholder. */
function MetaCell({
  value,
  title,
  align,
}: {
  value: string | null;
  title?: string;
  align?: "right";
}) {
  return (
    <span
      className={cn(
        "px-3",
        "py-1",
        "tabular-nums",
        "whitespace-nowrap",
        "truncate",
        align === "right" && "text-right",
        value === null ? "text-info-dim" : "text-info-mid",
      )}
      title={title}
    >
      {value ?? "—"}
    </span>
  );
}

/**
 * One tree node (and, when open, its subtree). `memo` + the fold's
 * identity discipline = a node re-renders only when ITS subtree
 * changed. `open` is deliberately LOCAL state: no shared Set whose
 * identity would defeat the memo; expansion survives updates (same
 * component instance, keyed by name) and dies with deleted nodes.
 */
const TreeNode = memo(function TreeNode({
  node,
  depth,
  sort,
}: {
  node: FileTreeNode;
  depth: number;
  sort: Sort;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const isDirectory = node.type === "directory";
  const entries = useMemo(
    () => (node.type === "directory" ? sortEntries(node.children, sort) : null),
    [node, sort],
  );

  const indent = { paddingLeft: `${0.75 + depth * 1.25}rem` };
  return (
    <>
      <div
        data-entry={node.name}
        className={cn(
          "grid",
          GRID_COLS,
          "border-b",
          "border-copper-mid/15",
          isDirectory && cn("cursor-pointer", "hover:bg-copper-warm/10"),
        )}
        onClick={isDirectory ? () => setOpen((o) => !o) : undefined}
      >
        <span
          className={cn("py-1", "pr-3", "truncate", "whitespace-nowrap")}
          style={indent}
        >
          <span className={cn("inline-block", "w-4", "text-info-dim")}>
            {isDirectory ? (open ? "▾" : "▸") : ""}
          </span>
          {node.type === "directory" && (
            <span className={cn("text-copper-bright")}>{node.name}/</span>
          )}
          {node.type === "file" && (
            <span className={cn("text-[#c3bfbb]")}>{node.name}</span>
          )}
          {node.type === "symlink" && (
            <span className={cn("text-info-mid", "italic")}>
              {node.name} →{node.target != null && <> {node.target}</>}
            </span>
          )}
        </span>
        <MetaCell
          align="right"
          value={
            node.type === "file" && node.size != null
              ? formatBytes(node.size)
              : null
          }
        />
        <MetaCell
          value={node.created_at != null ? formatTimestamp(node.created_at) : null}
        />
        <MetaCell
          value={
            node.modified_at != null ? formatTimestamp(node.modified_at) : null
          }
        />
        <MetaCell value={node.created_by ?? null} title={node.created_by ?? undefined} />
        <MetaCell
          value={node.modified_by ?? null}
          title={node.modified_by ?? undefined}
        />
      </div>
      {isDirectory && open && entries !== null && entries.length === 0 && (
        <div
          className={cn(
            "py-1",
            "text-info-dim",
            "italic",
            "border-b",
            "border-copper-mid/15",
          )}
          style={{ paddingLeft: `${0.75 + (depth + 1) * 1.25 + 1}rem` }}
        >
          empty
        </div>
      )}
      {isDirectory &&
        open &&
        entries !== null &&
        entries.map((child) => (
          <TreeNode key={child.name} node={child} depth={depth + 1} sort={sort} />
        ))}
    </>
  );
});

/** Centered placeholder shared by the connecting / no-files states. */
function CenterState({
  primary,
  secondary,
}: {
  primary: string;
  secondary?: string;
}) {
  return (
    <div
      className={cn(
        "flex-1",
        "flex",
        "flex-col",
        "items-center",
        "justify-center",
        "gap-3",
        "select-none",
      )}
    >
      <LogoMark className={cn("h-24", "w-auto", "text-info-dim/15")} />
      <span className={cn("text-sm", "text-info-dim")}>{primary}</span>
      {secondary && (
        <span className={cn("text-xs", "text-info-dim/70")}>{secondary}</span>
      )}
    </div>
  );
}

/**
 * The window body: the tree, edge to edge — the laboratory's identity
 * lives in the window TITLE (`{os}/{machine id}/{lab id}`), no header
 * bar. `children === null` = connecting; `[]` = the empty snapshot —
 * the container hasn't started (it starts lazily) or has no files: a
 * real state, NOT an error.
 */
export function LaboratoryBrowser({
  transport,
  id,
  machine,
  machineState,
}: {
  transport: ViewerTransport | null;
  id: string;
  machine?: string;
  machineState?: string;
}): ReactElement {
  const children = useLaboratoryFiletree(transport, id, machine, machineState);
  // One sorter at a time; a header click cycles ITS column asc → desc
  // → off (and switching columns starts at asc).
  const [sort, setSort] = useState<Sort>(null);
  const cycleSort = (column: SortColumn) =>
    setSort((current) =>
      current === null || current.column !== column
        ? { column, descending: false }
        : current.descending
          ? null
          : { column, descending: true },
    );

  return (
    <div className={cn("flex-1", "min-h-0", "flex", "flex-col", "font-mono")}>
      {children === null ? (
        <CenterState primary="connecting to laboratory…" />
      ) : children.length === 0 ? (
        <CenterState
          primary="no files yet"
          secondary="files appear when the laboratory container starts"
        />
      ) : (
        <div className={cn("flex-1", "min-h-0", "overflow-auto", "text-xs")}>
          <div
            className={cn(
              "grid",
              GRID_COLS,
              "sticky",
              "top-0",
              "bg-ground-surface",
              "border-b",
              "border-copper-mid/40",
              "text-info-dim",
            )}
          >
            {SORT_COLUMNS.map((column) => (
              <button
                key={column}
                type="button"
                onClick={() => cycleSort(column)}
                className={cn(
                  "px-3",
                  "py-1.5",
                  "text-left",
                  "cursor-pointer",
                  "select-none",
                  "hover:text-copper-bright",
                  column === "size" && "text-right",
                  sort?.column === column && "text-copper-bright",
                )}
              >
                {column}
                {sort?.column === column && (sort.descending ? " ↓" : " ↑")}
              </button>
            ))}
          </div>
          {sortEntries(children, sort).map((node) => (
            <TreeNode key={node.name} node={node} depth={0} sort={sort} />
          ))}
        </div>
      )}
    </div>
  );
}
