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
export interface ViewerOpenTab {
  /** Component module path, relative to the sender identity's root
   * (e.g. `/tabs/agents.js`). Absolute-from-root, no scheme, no
   * traversal — the shell rejects anything else. */
  module: string;
  /** The export holding the component (default `"default"`). */
  export?: string;
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
}

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
 * Close a viewer tab: the CALLING tab when `tab` is omitted (the
 * self-close every component may use — a spawned window's sole tab
 * closes the window with it), or the tab with that id. Either way
 * the shell's closability rules apply.
 */
export async function closeViewerTab(
  transport: ViewerTransport,
  tab?: number,
): Promise<void> {
  if (tab === undefined) {
    await transport.invoke("tabs_close_self", {});
  } else {
    await transport.invoke("tabs_close", { tabId: tab });
  }
}
