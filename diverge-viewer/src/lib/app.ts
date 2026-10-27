// The page's door to Rust for the app's own things: one typed wrapper per
// registry action. Types come from src/bindings (generated from Rust —
// never edit them). The daemon is not here: the page reaches it itself,
// with the SDK, through lib/daemon.ts.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ActionInfo } from "../bindings/ActionInfo";
import type { AppInfo } from "../bindings/AppInfo";
import type { FileNoticeView } from "../bindings/FileNoticeView";
import type { SavedView } from "../bindings/SavedView";
import type { TabKind } from "../bindings/TabKind";
import type { TabsSnapshot } from "../bindings/TabsSnapshot";
import type { Tileset } from "../bindings/Tileset";
import type { TilesetEntry } from "../bindings/TilesetEntry";

export const app = {
  info: () => invoke<AppInfo>("app_info"),
  filesSetAside: () => invoke<FileNoticeView[]>("files_set_aside"),
  actions: () => invoke<ActionInfo[]>("actions_list"),

  machineNames: () => invoke<Record<string, string>>("machines_names"),
  machineRename: (key: string, name: string) => invoke<Record<string, string>>("machines_rename", { key, name }),

  theme: () => invoke<string | null>("theme_get"),
  themeSet: (theme: string | null) => invoke<string | null>("theme_set", { theme }),

  tilesets: () => invoke<TilesetEntry[]>("tilesets_list"),
  tilesetLoad: (id: string) => invoke<Tileset>("tileset_load", { id }),
  // The atlas's bytes come raw, as an ArrayBuffer.
  tilesetImage: (id: string) => invoke<ArrayBuffer>("tileset_image", { id }),
  tileset: () => invoke<string>("tileset_get"),
  tilesetSet: (tileset: string | null) => invoke<string>("tileset_set", { tileset }),

  views: () => invoke<SavedView[]>("views_list"),
  viewSave: (view: SavedView) => invoke<SavedView>("views_save", { view }),
  viewDelete: (id: string) => invoke<null>("views_delete", { id }),

  tabs: () => invoke<TabsSnapshot>("tabs_snapshot"),
  tabOpen: (tab: TabKind) => invoke<TabsSnapshot>("tabs_open", { tab }),
  tabClose: (key: string) => invoke<TabsSnapshot>("tabs_close", { key }),
  onTabs: (handler: (snapshot: TabsSnapshot) => void) => listen<TabsSnapshot>("tabs://changed", (event) => handler(event.payload)),
};

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return JSON.stringify(error);
}
