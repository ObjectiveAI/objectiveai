/**
 * Shared node ID generation for both structural and execution trees.
 * Both must use the same scheme so animated transitions work correctly.
 */
export function nodeId(prefix: string, path: number[]): string {
  return path.length > 0 ? `${prefix}-${path.join("-")}` : prefix;
}
