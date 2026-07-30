// The plugins tab's invoke surface: list every installed plugin
// VERSION, install a new one (owner/name/version — the version is
// the plugin repo's v-prefixed git tag, Go-modules style), and
// uninstall an exact version. All three are root-identity commands;
// install/uninstall resolve only when the whole Rust pipeline has
// finished (fetch → pnpm → esbuild → land / unload → delete →
// prune), so callers drive busy state off the promise.

import { tauriInvoke } from "./tauri";

/** One installed plugin version — mirrors the shell's
 * `PluginVersionInfo`. */
export interface PluginVersionInfo {
  owner: string;
  name: string;
  /** The exact version dir name — the v-prefixed git tag
   * (`v1.2.3`). */
  version: string;
  description?: string;
  /** Whether the manifest declares a viewer extension. */
  hasViewer: boolean;
  /** Whether this plugin (any version) is registered for development.
   * Install and uninstall are gated while it is. */
  development: boolean;
}

export function pluginsList(): Promise<PluginVersionInfo[] | undefined> {
  return tauriInvoke<PluginVersionInfo[]>("plugins_list");
}

export async function pluginsInstall(
  owner: string,
  name: string,
  version: string,
): Promise<void> {
  await tauriInvoke("plugins_install", { owner, name, version });
}

export async function pluginsUninstall(
  owner: string,
  name: string,
  version: string,
): Promise<void> {
  await tauriInvoke("plugins_uninstall", { owner, name, version });
}
