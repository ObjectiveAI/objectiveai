// The app's transfers, asked from the page: a download saved where the
// person says, an upload of what they dropped. The bytes never come
// here; the app's Rust moves them between the disk and the wire.

import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { type Route, relayed } from "../lib/wire";
import type { Files } from "../world/model";

/** How a transfer ended, as the app says it. */
export type Outcome =
  | { kind: "done"; bytes: number }
  | { kind: "cancelled" }
  | { kind: "not_found" }
  | { kind: "forbidden" }
  | { kind: "held" }
  | { kind: "exists"; path: string }
  | { kind: "failed"; why: string };

/** The page's own number for each transfer, which cancels it. */
let next = 0;

/** A transfer under way: its number, and how it ends. */
export type Running = { id: number; ended: Promise<Outcome> };

/** The bytes so far, told as they move. */
function counting(progress: (bytes: number) => void): Channel<number> {
  const channel = new Channel<number>();
  channel.onmessage = progress;
  return channel;
}

/** A failure to ask at all, as an outcome. */
const refused = (error: unknown): Outcome => ({ kind: "failed", why: error instanceof Error ? error.message : String(error) });

/** Download what is at `path` in `files`, on the daemon `route` reaches; the app asks where to save it. */
export function download(files: Files, route: Route, path: string[], directory: boolean, progress: (bytes: number) => void): Running {
  const id = ++next;
  const ended = invoke<Outcome>("files_download", { id, files, path, directory, progress: counting(progress), ...relayed(route) }).catch(refused);
  return { id, ended };
}

/** Upload the dropped `paths` into the directory `into` in `files`, on the daemon `route` reaches. */
export function upload(files: Files, route: Route, into: string[], paths: string[], progress: (bytes: number) => void): Running {
  const id = ++next;
  const ended = invoke<Outcome>("files_upload", { id, files, into, paths, progress: counting(progress), ...relayed(route) }).catch(refused);
  return { id, ended };
}

/** Stop a transfer; one that is over is nothing. */
export function cancel(id: number): void {
  void invoke("files_cancel", { id }).catch(() => undefined);
}

/** Where a drag is, in the page's own pixels, and what it carries. */
export type Drag =
  | { kind: "over"; x: number; y: number; paths: string[] | null }
  | { kind: "drop"; x: number; y: number; paths: string[] }
  | { kind: "leave" };

/**
 * Hear every file dragged over the window. The window's own drop
 * handler takes files dragged in, so the page hears them only as these
 * events, their positions in the window's physical pixels.
 */
export async function onDrag(heard: (drag: Drag) => void): Promise<() => void> {
  const scale = window.devicePixelRatio || 1;
  return getCurrentWebview().onDragDropEvent((event) => {
    const drag = event.payload;
    switch (drag.type) {
      case "enter":
        heard({ kind: "over", x: drag.position.x / scale, y: drag.position.y / scale, paths: drag.paths });
        break;
      case "over":
        heard({ kind: "over", x: drag.position.x / scale, y: drag.position.y / scale, paths: null });
        break;
      case "drop":
        heard({ kind: "drop", x: drag.position.x / scale, y: drag.position.y / scale, paths: drag.paths });
        break;
      case "leave":
        heard({ kind: "leave" });
        break;
    }
  });
}
