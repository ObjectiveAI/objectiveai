// The TS mirror of the Rust shell model types
// (src-tauri/src/shell/model.rs) and the invoke helpers the chrome
// drives it with. The model is the single source of truth: every
// mutation broadcasts a full `tabs://changed` snapshot with a bumped
// generation; consumers apply a payload (event OR snapshot response)
// only when its generation advances, so a stale snapshot can never
// clobber a newer event. UI state (zoom/orientation) is per WINDOW,
// rides targeted `ui://changed` events instead of the snapshot, and
// is bridged chrome → content via `ui_set`/`ui_get`.
//
// A tab's KIND is the uniform component-coordinates shape — Rust
// knows no tab names, ours included: the chrome seeds the home tabs
// through the same `tabs_open` (the SDK's `openViewerTab`) every
// identity uses, and content webviews learn who they are via
// `tab_self`.

import { openViewerTab, type ViewerOpenTab } from "@objectiveai/sdk";
import { tauriInvoke } from "./tauri";
import { viewerTransport } from "./viewer-transport";

/** What a tab IS — identity + component coordinates + opaque props.
 * Kind equality (all four fields) is the shell's dedupe key. */
export interface TabKind {
  identity: string;
  module: string;
  export?: string;
  arguments?: unknown;
}

export interface TabDesc {
  id: number;
  kind: TabKind;
  title: string;
  closable: boolean;
  /** Optional identity icon, identity-root-relative (resolve with
   * [`tabIconUrl`]). */
  icon?: string;
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

/** What a content webview learns about itself at boot — the generic
 * bootstrap's whole input. */
export interface TabDescriptor {
  identity: string;
  module: string;
  export?: string;
  arguments?: unknown;
  title: string;
}

/** A built-in tab component's module path: vite source paths in dev
 * (the dev server transpiles on demand), the stably-named built
 * chunks in production. JS owns this knowledge — Rust knows no
 * module paths. */
export function builtinTabModule(stem: string): string {
  return import.meta.env.DEV ? `/src/tabs/${stem}.tsx` : `/tabs/${stem}.js`;
}

/** The home tabs the boot chrome seeds (in strip order), through the
 * same open API every identity uses. */
export const HOME_TABS = [
  "agents",
  "laboratories",
  "viewer-logs",
  "command-logs",
] as const;

export function tabsSnapshot(): Promise<TabsSnapshot | undefined> {
  return tauriInvoke<TabsSnapshot>("tabs_snapshot");
}

/** The root identity's icon — what a plugin's manifest icon is to
 * it; every objectiveai open call defaults to this. The white-glyph
 * variant of the mark (matching the bundled window icon) — the
 * favicon stays the copper original. */
export const OBJECTIVEAI_ICON = "/icon.svg";

/** Resolve a tab's identity-root-relative icon path to a URL the
 * CHROME can render. The root identity shares the chrome's origin,
 * so the path serves as-is; a plugin identity's root is its own
 * origin — prefixing lands here when plugin identities exist. */
export function tabIconUrl(tab: TabDesc): string | undefined {
  return tab.icon;
}

/** Open (or focus) a tab — the SDK helper over the shell's
 * `tabs_open`; the sender's identity is derived Rust-side from THIS
 * webview. Every open from OUR code carries the objectiveai icon
 * unless the caller says otherwise (a plugin passes its manifest
 * icon — or nothing — through the SDK helper directly). */
export async function tabsOpen(tab: ViewerOpenTab): Promise<void> {
  const transport = await viewerTransport();
  if (transport !== null) {
    await openViewerTab(transport, { icon: OBJECTIVEAI_ICON, ...tab });
  }
}

/** Seed the home tabs into an EMPTY shell (the boot chrome calls
 * this once, when the whole model holds zero tabs), then hand focus
 * back to the first of them. */
export async function seedHomeTabs(): Promise<void> {
  for (const stem of HOME_TABS) {
    await tabsOpen({
      module: builtinTabModule(stem),
      title: stem,
      closable: false,
    });
  }
  // Each open activated its own tab — re-activate the FIRST home
  // tab (it lives in this chrome's window; select acts there).
  const snapshot = await tabsSnapshot();
  if (!snapshot) return;
  const first = builtinTabModule(HOME_TABS[0]);
  for (const windowTabs of Object.values(snapshot.windows)) {
    const tab = windowTabs.tabs.find((t) => t.kind.module === first);
    if (tab) {
      tabsSelect(tab.id);
      return;
    }
  }
}

/** This content webview's own descriptor. */
export function tabSelf(): Promise<TabDescriptor | undefined> {
  return tauriInvoke<TabDescriptor>("tab_self");
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
