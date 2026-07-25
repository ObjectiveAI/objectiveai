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

import { convertFileSrc } from "@tauri-apps/api/core";
import { closeViewerTab, openViewerTab, type ViewerOpenTab } from "@objectiveai/sdk";
import { isTauri, tauriInvoke } from "./tauri";
import { viewerTransport } from "./viewer-transport";

/** Mirror of Rust's `model.rs` `ROOT_IDENTITY` — the identity whose
 * module/icon paths resolve against the app origin as-is. */
export const ROOT_IDENTITY = "objectiveai";

/** A plugin asset's platform-correct URL. `convertFileSrc`
 * percent-encodes a real path (slashes become `%2F`), but with an
 * EMPTY path it yields exactly the plugin origin + `/` —
 * `http://plugin.localhost/` on Windows/Android, `plugin://localhost/`
 * elsewhere — so append `{identity}{path}` verbatim (identity has no
 * leading slash; the normalized path has one). */
export function pluginAssetUrl(identity: string, path: string): string {
  return convertFileSrc("", "plugin") + identity + path;
}

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
  /** `true` = the module is ROOT code despite a plugin identity (the
   * channel-request template) — no plugin origin prefixing. */
  rootModule?: boolean;
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

/** The root identity's icon — what a plugin's manifest icon is to
 * it; every objectiveai open call defaults to this. The white-glyph
 * variant of the mark (matching the bundled window icon) — the
 * favicon stays the copper original. */
export const OBJECTIVEAI_ICON = "/icon.svg";

/** One declared root tab — mirrors the shell's `DeclareEntry`. */
export interface DeclareTabEntry {
  name: string;
  title: string;
  module: string;
  export?: string;
  icon?: string;
  closable: boolean;
  permanent?: boolean;
}

/** The ROOT identity's tab inventory, in strip order — the chrome's
 * manifest-equivalent (Rust hardcodes no names; this list is how it
 * learns ours). The `tabs` tab is PERMANENT: always open, greyed in
 * its own list, never toggleable. */
const ROOT_TABS: DeclareTabEntry[] = [
  ...(
    ["agents", "laboratories", "viewer-logs", "command-logs", "plugins"] as const
  ).map(
    (stem) => ({
      name: stem,
      title: stem,
      module: builtinTabModule(stem),
      icon: OBJECTIVEAI_ICON,
      closable: false,
    }),
  ),
  {
    name: "tabs",
    title: "tabs",
    module: builtinTabModule("tabs"),
    icon: OBJECTIVEAI_ICON,
    closable: false,
    permanent: true,
  },
];

/** Declare the root inventory — every chrome calls this on mount;
 * Rust applies the FIRST declaration per app run and no-ops the
 * rest. The boot orchestrator opens the enabled slice. */
export async function declareTabs(): Promise<void> {
  await tauriInvoke("tabs_declare", { entries: ROOT_TABS });
}

/** Declare the channel-request tab TEMPLATE — the component the
 * shell's resident /channels listener spawns for every incoming
 * offer. Same first-wins contract as `declareTabs`; the listener
 * won't connect until a chrome has declared (dev/prod module
 * knowledge stays here — Rust knows no module paths). */
export async function declareChannelRequestTab(): Promise<void> {
  await tauriInvoke("channel_request_declare", {
    module: builtinTabModule("channel-request"),
  });
}

/** Close this content webview's OWN tab (the channel-request tab's
 * Decline) — the SDK helper over the shell's `tabs_close_self`. The
 * sole tab of a spawned window closes the window with it. */
export function tabsCloseSelf(): void {
  void (async () => {
    const transport = await viewerTransport();
    if (transport !== null) {
      await closeViewerTab(transport);
    }
  })();
}

/** Accept the CALLING channel-request tab's offer (self-scoped).
 * Rust runs the accept POST, spawns the publishing plugin's handler
 * component focused, and closes this tab — on failure it closes this
 * tab too, so the caller only shows a loading state and waits to
 * die. */
export function channelRequestAccept(): Promise<void> {
  return tauriInvoke("channel_request_accept").then(() => undefined);
}

/** The CALLING request tab's offer standing (self-scoped): what verb
 * the tab renders. */
export type OfferStatus =
  | "ready"
  | "not_installed"
  | "unsupported_key"
  | "no_plugin";

export function channelRequestStatus(): Promise<OfferStatus | undefined> {
  return tauriInvoke<OfferStatus>("channel_request_status");
}

/** One coarse install phase from the offer tab's Install run. */
export type ChannelInstallStep = { step: "building" | "installing" };

/** Install the CALLING request tab's publishing plugin (self-scoped)
 * — the daemon builds, this side lands. `onStep` follows the two
 * phases; resolve = installed (re-query the status), reject = Rust is
 * closing this tab. */
export async function channelRequestInstall(
  onStep: (step: ChannelInstallStep) => void,
): Promise<void> {
  const transport = await viewerTransport();
  if (transport === null) {
    throw new Error("not running under Tauri");
  }
  const channel = transport.channel<ChannelInstallStep>();
  channel.onmessage = onStep;
  await transport.invoke("channel_request_install", { onStep: channel });
}

/** One inventory row — mirrors the shell's `InventoryEntry`. */
export interface TabInventoryEntry {
  identity: string;
  identityKey: string;
  name: string;
  title: string;
  module: string;
  export?: string;
  icon?: string;
  closable: boolean;
  permanent: boolean;
  enabled: boolean;
}

export function tabsInventory(): Promise<TabInventoryEntry[] | undefined> {
  return tauriInvoke<TabInventoryEntry[]>("tabs_inventory");
}

export function tabsToggle(
  identityKey: string,
  name: string,
  enabled: boolean,
): void {
  void tauriInvoke("tabs_toggle", { identityKey, name, enabled });
}

/** One (identityKey, name) pair in a display order. */
export interface TabOrderRef {
  identityKey: string;
  name: string;
}

/** Persist a new tab order (the FULL display order — every loaded
 * entry, enabled and disabled). Awaitable: a rejection (stale order
 * raced a rescan) lets the pane re-fetch and self-heal. Outside
 * user-controlled mode the live strip follows. */
export async function tabsReorder(order: TabOrderRef[]): Promise<void> {
  await tauriInvoke("tabs_reorder", { order });
}

export function tabsSnapshot(): Promise<TabsSnapshot | undefined> {
  return tauriInvoke<TabsSnapshot>("tabs_snapshot");
}

/** Resolve an identity-root-relative icon path to a URL the CHROME
 * can render. The root identity shares the chrome's origin, so the
 * path serves as-is; a plugin identity's icon is always a plugin
 * asset (no root-template case exists for icons), served through the
 * plugin:// protocol. */
export function tabIconUrl(
  identity: string,
  icon: string | undefined,
): string | undefined {
  if (icon === undefined || identity === ROOT_IDENTITY || !isTauri()) {
    return icon;
  }
  return pluginAssetUrl(identity, icon);
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

/** This content webview's own descriptor. */
export function tabSelf(): Promise<TabDescriptor | undefined> {
  return tauriInvoke<TabDescriptor>("tab_self");
}

export function tabsSelect(tabId: number): void {
  void tauriInvoke("tabs_select", { tabId });
}

export function tabsClose(tabId: number): void {
  void (async () => {
    const transport = await viewerTransport();
    if (transport !== null) {
      await closeViewerTab(transport, tabId);
    }
  })();
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
