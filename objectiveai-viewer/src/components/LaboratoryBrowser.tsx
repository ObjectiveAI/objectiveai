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

/** Directories first, then name order, per level. */
function sortEntries(nodes: FileTreeNode[]): FileTreeNode[] {
  return [...nodes].sort(
    (a, b) =>
      (a.type === "directory" ? 0 : 1) - (b.type === "directory" ? 0 : 1) ||
      a.name.localeCompare(b.name),
  );
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
}: {
  node: FileTreeNode;
  depth: number;
}): ReactElement {
  const [open, setOpen] = useState(false);
  const isDirectory = node.type === "directory";
  const entries = useMemo(
    () => (node.type === "directory" ? sortEntries(node.children) : null),
    [node],
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
              {node.name} →
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
          <TreeNode key={child.name} node={child} depth={depth + 1} />
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
 * The window body: header bar (laboratory id + host), then the tree.
 * `children === null` = connecting; `[]` = the empty snapshot — the
 * container hasn't started (it starts lazily) or has no files: a real
 * state, NOT an error.
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

  return (
    <div className={cn("flex-1", "min-h-0", "flex", "flex-col", "font-mono")}>
      <div
        className={cn(
          "shrink-0",
          "flex",
          "items-baseline",
          "gap-2",
          "px-3",
          "py-1.5",
          "border-b",
          "border-copper-mid/40",
          "text-xs",
        )}
      >
        <span className={cn("text-info-bright")}>{id}</span>
        {(machine != null || machineState != null) && (
          <span className={cn("text-info-dim", "truncate")} title={machine}>
            {machine != null ? `${machine.slice(0, 16)}…` : "any host"}
            {machineState != null ? ` · ${machineState}` : ""}
          </span>
        )}
      </div>
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
            <span className={cn("px-3", "py-1.5", "pl-3")}>name</span>
            <span className={cn("px-3", "py-1.5", "text-right")}>size</span>
            <span className={cn("px-3", "py-1.5")}>created</span>
            <span className={cn("px-3", "py-1.5")}>modified</span>
            <span className={cn("px-3", "py-1.5")}>created by</span>
            <span className={cn("px-3", "py-1.5")}>modified by</span>
          </div>
          {sortEntries(children).map((node) => (
            <TreeNode key={node.name} node={node} depth={0} />
          ))}
        </div>
      )}
    </div>
  );
}
