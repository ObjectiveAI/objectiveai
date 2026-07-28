/**
 * The viewer-UI surface: opening tabs in the viewer shell. Distinct
 * from the daemon transports — this is the shell's own command, not a
 * daemon endpoint (the daemon-side viewer plumbing lives in
 * `daemon/viewerStream.ts`, sharing the same injected
 * {@link ViewerTransport}).
 */

import type { ViewerTransport } from "./daemon/viewerStream";

/**
 * One tab open request — the argument to the viewer shell's
 * `tabs_open` command. The SENDER's identity is deliberately NOT
 * part of it: the Rust shell derives identity from the calling
 * webview (the root chrome and built-in tabs are `objectiveai`; a
 * plugin's webviews are that plugin) and resolves `module` against
 * that identity's root — a caller can only ever open tabs whose code
 * lives under its own root.
 */
/** A tab rendering one of your components — the ordinary kind. */
export interface ViewerOpenComponentTab extends ViewerOpenTabCommon {
  /** Component module path, relative to the sender identity's root
   * (e.g. `/tabs/agents.js`). Absolute-from-root, no scheme, no
   * traversal — the shell rejects anything else. */
  module: string;
  /** The export holding the component (default `"default"`). */
  export?: string;
  url?: never;
  script?: never;
  state?: never;
}

/**
 * A tab rendering a real web page in a real Chromium browser, opened
 * inside the viewer as an ordinary tab: it sits in the strip, moves
 * between windows, and closes like any other.
 *
 * It is NOT one of your components — there is no module, no props and
 * no bootstrap, so none of the viewer SDK is available to the page.
 * The only code of yours that runs in it is `script`.
 */
export interface ViewerOpenBrowserTab extends ViewerOpenTabCommon {
  /** Where the browser opens. */
  url: string;
  /** OPTIONAL name of one of your plugin manifest's `scripts`,
   * injected into every top-level page load (never into iframes).
   *
   * A NAME, not code: the runnable set is closed to what the plugin
   * declared at install time. */
  script?: string;
  /** OPTIONAL profile key. Supply one and the browser's cookies,
   * localStorage and cache PERSIST on disk — so a sign-in survives
   * closing the tab and is still there next time you open it under
   * the same key. Omit it and the browser is entirely in memory and
   * forgets everything on close.
   *
   * The profile belongs to your identity and this key together, so
   * different keys are different logins and no other plugin can reach
   * either. Only ONE browser may drive a given profile at a time —
   * opening a second on the same key fails rather than corrupting the
   * store the first is writing.
   *
   * Part of the dedupe identity: two browsers on the same URL with
   * different profiles are genuinely different tabs. */
  state?: string;
  module?: never;
  export?: never;
}

/** What every tab has, whichever surface it renders. */
interface ViewerOpenTabCommon {
  /** The tab's display title. */
  title: string;
  /** Opaque props delivered verbatim to the component at boot. */
  arguments?: unknown;
  /** Whether the strip shows a close button (default `true`). */
  closable?: boolean;
  /** OPTIONAL identity icon, shown beside the identity in the strip
   * — a path relative to the sender identity's root, same rules as
   * `module` (a plugin's manifest icon; omit for none). */
  icon?: string;
  /** OPTIONAL stylesheets, as paths relative to the sender
   * identity's root (same rules as `module`). The shell injects each
   * as a `<link rel="stylesheet">` and WAITS for it before the
   * component renders — so no flash of unstyled content, and a sheet
   * that fails to load stops the tab rather than showing it wrong.
   *
   * Needed because a bundler strips `import "./x.css"` from a JS
   * entry and emits the file beside it: nothing would ever request
   * it. Cosmetic, like `icon` — not part of the dedupe identity. */
  styles?: string[];
  /** OPTIONAL name for the spawned tab, unique among THIS tab's
   * children. It is the address the parent messages the child at
   * afterwards ([`sendViewerTab`] and friends); the child answers
   * with [`sendViewerParent`] and needs no key of its own.
   *
   * Unlike the cosmetic fields above this IS part of the dedupe
   * identity: opening the same component under a different key gives
   * you a genuinely different tab, so one parent can spawn several
   * children of one component and address them separately. */
  key?: string;
}

/** One tab open request — a component tab or a browser tab. */
export type ViewerOpenTab = ViewerOpenComponentTab | ViewerOpenBrowserTab;

/**
 * Open a viewer tab in the calling window — or focus it, wherever it
 * lives, if one with the same identity + module + export + arguments
 * already exists (the shell's open-or-focus dedupe; `title` and
 * `closable` are cosmetic, not identity). Resolves once the shell
 * has applied the open.
 */
export async function openViewerTab(
  transport: ViewerTransport,
  tab: ViewerOpenTab,
): Promise<void> {
  await transport.invoke("tabs_open", { tab });
}

/**
 * Close a viewer tab. Three targets:
 *
 * - **omitted** — the CALLING tab (the self-close every component may
 *   use; a spawned window's sole tab closes the window with it).
 * - **`{ key }`** — the tab this one spawned under that name. The only
 *   SCOPED form: the key resolves through the caller's own mailbox
 *   index, so a tab can close a tab it spawned and nothing else. A
 *   child that has already closed is not an error.
 * - **a tab id** — that exact tab, whoever owns it. Intended for the
 *   chrome, which renders the snapshot and legitimately knows every
 *   id; a component has no sanctioned way to learn another tab's id
 *   and should prefer `{ key }`.
 *
 * Either way the shell's closability rules apply.
 */
export async function closeViewerTab(
  transport: ViewerTransport,
  tab?: number | { key: string },
): Promise<void> {
  if (tab === undefined) {
    await transport.invoke("tabs_close_self", {});
  } else if (typeof tab === "number") {
    await transport.invoke("tabs_close", { tabId: tab });
  } else {
    await transport.invoke("tabs_close_child", { key: tab.key });
  }
}

/**
 * Messaging between a tab and the tabs it spawned.
 *
 * Every tab is its own webview, so the only thing a parent could ever
 * hand a child was `arguments`, delivered once at boot. These keep the
 * two talking: name a tab when you open it (`ViewerOpenTab.key`) and
 * the pair share a mailbox with a lane in each direction.
 *
 * The parent addresses a child by that key; the child answers with the
 * `...Parent` calls and supplies no key at all — the shell knows who
 * spawned it, so a child can neither name nor misname anyone.
 *
 * A mailbox outlives either tab alone: it accepts sends, subscribes
 * and lists until BOTH ends are closed. Sends queue unconditionally,
 * so a parent may send the instant it spawns and the child drains it
 * on its first subscribe.
 */

/** Queue a value for the tab this one spawned as `key`. */
export async function sendViewerTab(
  transport: ViewerTransport,
  key: string,
  value: unknown,
): Promise<void> {
  await transport.invoke("tabs_send", { key, value });
}

/**
 * Everything that child has sent and this tab has not yet seen.
 *
 * Returns IMMEDIATELY when anything is pending, and never yields the
 * same item twice. Blocks ONLY on an empty lane — until something
 * arrives, until the child closes (a wait never outlives the tab it
 * waits on), or until `timeoutMs` elapses; omit it to wait forever.
 * Once the child is closed this stops blocking, so a bare `while
 * (true)` loop will spin — drive it off your own condition.
 */
export async function subscribeViewerTab(
  transport: ViewerTransport,
  key: string,
  timeoutMs?: number,
): Promise<unknown[]> {
  return (await transport.invoke("tabs_subscribe", {
    key,
    timeoutMs,
  })) as unknown[];
}

/**
 * That child's retained history (capped at the most recent 1024).
 * `pending` drains, advancing the cursor exactly like a non-blocking
 * subscribe; otherwise the full history comes back and nothing is
 * marked read — the `--pending` / `--all` split the channel and agent
 * log commands take.
 */
export async function listViewerTab(
  transport: ViewerTransport,
  key: string,
  pending = false,
): Promise<unknown[]> {
  return (await transport.invoke("tabs_list", { key, pending })) as unknown[];
}

/** Queue a value for the tab that spawned this one. */
export async function sendViewerParent(
  transport: ViewerTransport,
  value: unknown,
): Promise<void> {
  await transport.invoke("tabs_parent_send", { value });
}

/** [`subscribeViewerTab`] against the spawning tab. */
export async function subscribeViewerParent(
  transport: ViewerTransport,
  timeoutMs?: number,
): Promise<unknown[]> {
  return (await transport.invoke("tabs_parent_subscribe", {
    timeoutMs,
  })) as unknown[];
}

/** [`listViewerTab`] against the spawning tab. */
export async function listViewerParent(
  transport: ViewerTransport,
  pending = false,
): Promise<unknown[]> {
  return (await transport.invoke("tabs_parent_list", { pending })) as unknown[];
}
