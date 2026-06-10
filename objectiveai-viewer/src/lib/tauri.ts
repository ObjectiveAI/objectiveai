export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function tauriListen<T>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<() => void> {
  if (!isTauri()) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, handler);
}

export async function tauriInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | undefined> {
  if (!isTauri()) return undefined;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}
