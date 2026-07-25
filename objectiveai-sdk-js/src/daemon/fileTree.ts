/**
 * Materialized consumer of the cli daemon's
 * `/laboratories/{id}/filetree` endpoint — the JS mirror of the Rust
 * SDK's `daemon::file_tree::FileTree`, folded with the same event
 * semantics (`FileTreeEvent::apply` in `wire.rs`).
 *
 * The fold here is deliberately PATH-COPYING IMMUTABLE (the repo's
 * merge-system identity convention, unlike the Rust in-place fold):
 * every applied event produces a NEW root array, shallow-copying only
 * the spine from the root to the changed node — every untouched
 * sibling subtree keeps its object identity. React consumers can
 * therefore `memo` per-node components and re-render exactly the
 * changed region.
 *
 * One listener = one connection: when the stream closes the view
 * freezes at its last state; reconnect is the caller's loop — mint a
 * new listener from the client. Unparseable events are skipped
 * (forward compat).
 */

import type {
  LaboratoriesFiletreeFileTreeEvent,
  LaboratoriesFiletreeFileTreeNode,
} from "../laboratories/filetree/index";
import { openStream, type ClientMode } from "./mode";

type FileTreeNode = LaboratoriesFiletreeFileTreeNode;
type FileTreeEvent = LaboratoriesFiletreeFileTreeEvent;

/**
 * THE fold — the TS port of the Rust SDK's `FileTreeEvent::apply`,
 * path-copying instead of mutating: `snapshot` replaces the whole
 * child set; `upserted` rebuilds the spine down `path` and
 * replaces-or-appends the leaf by name (missing middle segments are
 * synthesized as empty directories, non-directories in the way are
 * replaced); `removed` rebuilds the spine and drops the leaf (a
 * missing path is a no-op returning the SAME root — no spurious
 * identity change). Idempotent, so at-least-once delivery is safe.
 */
export function applyFileTreeEvent(
  root: FileTreeNode[],
  event: FileTreeEvent,
): FileTreeNode[] {
  switch (event.type) {
    case "snapshot":
      return event.children;
    case "upserted":
      if (event.path.length === 0) return root;
      return upsertAt(root, event.path, 0, event.node);
    case "removed":
      if (event.path.length === 0) return root;
      return removeAt(root, event.path, 0);
    default:
      // Unknown event type — forward compat, no change.
      return root;
  }
}

/** Copy `children` with `node` installed at `path[depth..]`,
 * spine-copying directories on the way down. */
function upsertAt(
  children: FileTreeNode[],
  path: string[],
  depth: number,
  node: FileTreeNode,
): FileTreeNode[] {
  const name = path[depth];
  const next = [...children];
  const i = next.findIndex((child) => child.name === name);
  if (depth === path.length - 1) {
    if (i >= 0) next[i] = node;
    else next.push(node);
    return next;
  }
  const existing = i >= 0 ? next[i] : undefined;
  const directory: FileTreeNode =
    existing !== undefined && existing.type === "directory"
      ? {
          ...existing,
          children: upsertAt(existing.children, path, depth + 1, node),
        }
      : {
          // Missing middle (or a non-directory in the way) — a bare
          // synthesized directory, exactly like the Rust fold.
          type: "directory",
          name,
          children: upsertAt([], path, depth + 1, node),
        };
  if (i >= 0) next[i] = directory;
  else next.push(directory);
  return next;
}

/** Copy `children` with the node at `path[depth..]` removed,
 * spine-copying on the way down. Returns the SAME array when the
 * path doesn't resolve — no change, no identity churn. */
function removeAt(
  children: FileTreeNode[],
  path: string[],
  depth: number,
): FileTreeNode[] {
  const name = path[depth];
  const i = children.findIndex((child) => child.name === name);
  if (i < 0) return children;
  if (depth === path.length - 1) {
    const next = [...children];
    next.splice(i, 1);
    return next;
  }
  const existing = children[i];
  if (existing.type !== "directory") return children;
  const nextChildren = removeAt(existing.children, path, depth + 1);
  if (nextChildren === existing.children) return children;
  const next = [...children];
  next[i] = { ...existing, children: nextChildren };
  return next;
}

/**
 * The materialized `/laboratories/{id}/filetree` view — minted by
 * {@link Client.fileTree}; resolves only once the stream has OPENED.
 *
 * ```ts
 * const tree = await client.fileTree("my-lab");
 * tree.children(); // FileTreeNode[] | null (null until the first frame)
 * await tree.subscribe(); // resolves on the next change
 * ```
 */
export class FileTree {
  #abort: AbortController;
  #closed = false;
  /** The folded root child set — `null` until the first frame lands
   * (an unknown/not-yet-started laboratory sends an EMPTY snapshot,
   * which is a real state, distinct from "still connecting"). */
  #root: FileTreeNode[] | null = null;
  /** Resolvers for the in-flight {@link subscribe} promises. */
  #waiters = new Set<() => void>();

  private constructor(abort: AbortController) {
    this.#abort = abort;
  }

  /** @internal — minted by `Client.fileTree`. `laboratoryId` is the
   * raw laboratory id; the optional `(machine, machineState)` pair
   * pins the exact owning host (both or neither). */
  static async _connect(
    mode: ClientMode,
    laboratoryId: string,
    options?: { machine?: string; machineState?: string },
  ): Promise<FileTree> {
    const abort = new AbortController();
    const events = await openStream(mode, abort.signal, {
      route: `/laboratories/${laboratoryId}/filetree`,
      query: {
        machine: options?.machine,
        machine_state: options?.machineState,
      },
      viewerCommand: "daemon_laboratory_filetree",
      viewerArgs: {
        id: laboratoryId,
        machine: options?.machine,
        machineState: options?.machineState,
      },
    });
    const tree = new FileTree(abort);
    void tree.#pump(events);
    return tree;
  }

  /** Read the SSE stream, feeding each event to {@link #onFrame} until
   * it ends (or is aborted); then settle as closed. */
  async #pump(events: AsyncGenerator<string>): Promise<void> {
    try {
      for await (const data of events) {
        let frame: unknown;
        try {
          frame = JSON.parse(data);
        } catch {
          continue;
        }
        this.#onFrame(frame);
      }
    } catch {
      // Aborted or transport error — fall through to close.
    } finally {
      this.#onClose();
    }
  }

  /** Whether the connection has closed (the view is frozen). */
  get closed(): boolean {
    return this.#closed;
  }

  /** Drop the connection: the view freezes and any pending
   * {@link subscribe} resolves. */
  close(): void {
    if (this.#closed) return;
    this.#abort.abort();
    this.#onClose();
  }

  /** The folded root child set — `null` until the first frame. The
   * array (and every unchanged subtree in it) is identity-stable
   * across events; only changed spines get new objects. */
  children(): FileTreeNode[] | null {
    return this.#root;
  }

  /** Resolves on the next change applied to the state. A fresh call
   * waits for the FIRST change after it is made — loop with the
   * {@link children} read. Resolves immediately if already closed. */
  subscribe(): Promise<void> {
    if (this.#closed) return Promise.resolve();
    return new Promise<void>((resolve) => {
      this.#waiters.add(resolve);
    });
  }

  #onFrame(frame: unknown): void {
    if (typeof frame !== "object" || frame === null) return;
    const kind = (frame as { type?: unknown }).type;
    if (kind !== "snapshot" && kind !== "upserted" && kind !== "removed") {
      return;
    }
    this.#root = applyFileTreeEvent(
      this.#root ?? [],
      frame as FileTreeEvent,
    );
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }

  #onClose(): void {
    if (this.#closed) return;
    this.#closed = true;
    const waiters = [...this.#waiters];
    this.#waiters.clear();
    for (const wake of waiters) wake();
  }
}
