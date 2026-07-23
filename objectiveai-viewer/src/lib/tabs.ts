// The TS mirror of the Rust shell model types
// (src-tauri/src/shell/model.rs) and the invoke helpers the chrome
// drives it with. The model is the single source of truth: every
// mutation broadcasts a full `tabs://changed` snapshot with a bumped
// generation; consumers apply a payload (event OR snapshot response)
// only when its generation advances, so a stale snapshot can never
// clobber a newer event. UI state (zoom/orientation) is per WINDOW,
// rides targeted `ui://changed` events instead of the snapshot, and
// is bridged chrome → content via `ui_set`/`ui_get`.

import { tauriInvoke } from "./tauri";

export type TabKind =
  | { type: "agents" }
  | { type: "laboratories" }
  | { type: "agent"; aih: string }
  | {
      type: "laboratory";
      id: string;
      machine: string | null;
      machine_state: string | null;
      machine_os: string | null;
    };

export interface TabDesc {
  id: number;
  kind: TabKind;
  title: string;
  closable: boolean;
}

export interface WindowTabs {
  tabs: TabDesc[];
  /** The active tab's id (0 = none — an empty window). */
  active: number;
}

export interface TabsSnapshot {
  generation: number;
  windows: Record<string, WindowTabs>;
}

export function tabsSnapshot(): Promise<TabsSnapshot | undefined> {
  return tauriInvoke<TabsSnapshot>("tabs_snapshot");
}

export function tabsOpen(kind: TabKind): void {
  void tauriInvoke("tabs_open", { kind });
}

export function tabsSelect(tabId: number): void {
  void tauriInvoke("tabs_select", { tabId });
}

export function tabsClose(tabId: number): void {
  void tauriInvoke("tabs_close", { tabId });
}

export function tabsMove(tabId: number, index: number): void {
  void tauriInvoke("tabs_move", { tabId, index });
}

export function tabsDetach(tabId: number): void {
  void tauriInvoke("tabs_detach", { tabId });
}

/** One window's chrome-driven UI state, adopted by whichever content
 * webviews it currently hosts. */
export interface UiState {
  zoom: number;
  orientation: "vertical" | "horizontal";
}

export function uiSet(ui: Partial<UiState>): void {
  void tauriInvoke("ui_set", { ...ui });
}

export function uiGet(): Promise<UiState | undefined> {
  return tauriInvoke<UiState>("ui_get");
}
