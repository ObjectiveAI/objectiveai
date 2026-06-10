import type {
  FunctionListItem,
  FunctionDefinition,
  FunctionMeta,
} from "./types";
import { apiFetch } from "../client";

/** Simple TTL cache for the function list (shared across pages) */
let listCache: { data: FunctionListItem[]; ts: number } | null = null;
const LIST_TTL = 300_000; // 5 minutes

/** Immutable cache for GitHub definitions (keyed by commit SHA) */
const defCache = new Map<string, FunctionDefinition>();

/** Fetch the function list from the API (cached). Returns empty on failure. */
export async function fetchFunctionList(): Promise<FunctionListItem[]> {
  if (listCache && Date.now() - listCache.ts < LIST_TTL) {
    return listCache.data;
  }
  const data = await apiFetch<{ data: FunctionListItem[] }>("/functions");
  listCache = { data: data.data, ts: Date.now() };
  return listCache.data;
}

/** Fetch a function.json definition directly from GitHub (cached by commit) */
export async function fetchFunctionDefinition(
  owner: string,
  repository: string,
  commit: string
): Promise<FunctionDefinition> {
  const key = `${owner}/${repository}@${commit}`;
  const cached = defCache.get(key);
  if (cached) return cached;

  const url = `https://raw.githubusercontent.com/${owner}/${repository}/${commit}/function.json`;
  const res = await fetch(url);
  if (!res.ok)
    throw new Error(`Failed to fetch function.json: ${res.status} ${url}`);
  const def: FunctionDefinition = await res.json();
  defCache.set(key, def);
  return def;
}

/** Parse a function type string into category and depth */
export function parseType(type: string): {
  category: "scalar" | "vector";
  depth: "leaf" | "branch";
  clean: string;
} {
  const clean = type.replace(/^alpha\./, "").replace(/\.function$/, "");
  const parts = clean.split(".");
  return {
    category: (parts[0] as "scalar" | "vector") ?? "scalar",
    depth: (parts[1] as "leaf" | "branch") ?? "leaf",
    clean,
  };
}

/** Extract sub-function references from tasks */
function extractSubFunctions(tasks: FunctionDefinition["tasks"]): string[] {
  return tasks
    .filter((t) => t.type.includes(".function") && (t.repository || t.name))
    .map((t) => t.repository ?? t.name!);
}

/** Resolve a function list item into rich metadata */
export async function resolveFunctionMeta(
  item: FunctionListItem
): Promise<FunctionMeta> {
  const def = await fetchFunctionDefinition(
    item.owner,
    item.repository,
    item.commit
  );
  const { category, depth, clean } = parseType(def.type);

  return {
    ...item,
    name: item.repository,
    type: clean,
    category,
    depth,
    description: def.description ?? "",
    taskCount: def.tasks.length,
    subFunctions: extractSubFunctions(def.tasks),
  };
}

/** Recursively fetch a function and all its sub-function definitions */
export async function fetchRecursive(
  owner: string,
  repository: string,
  commit: string,
  defs: Map<string, FunctionDefinition>,
  commitMap: Map<string, string>
): Promise<void> {
  const key = `${owner}/${repository}`;
  if (defs.has(key)) return;

  try {
    const def = await fetchFunctionDefinition(owner, repository, commit);
    defs.set(key, def);

    const subFetches = def.tasks
      .filter((t) => t.type.includes("function") && t.owner && t.repository)
      .map((t) => {
        const subCommit =
          t.commit ?? commitMap.get(`${t.owner}/${t.repository}`) ?? "main";
        return fetchRecursive(t.owner!, t.repository!, subCommit, defs, commitMap);
      });

    await Promise.allSettled(subFetches);
  } catch {
    // Silently skip functions we can't fetch
  }
}

/** Fetch the full list with resolved metadata (parallel GitHub fetches) */
export async function fetchAllFunctions(): Promise<FunctionMeta[]> {
  const list = await fetchFunctionList();
  const results = await Promise.allSettled(
    list.map((item) => resolveFunctionMeta(item))
  );
  return results
    .filter(
      (r): r is PromiseFulfilledResult<FunctionMeta> =>
        r.status === "fulfilled"
    )
    .map((r) => r.value);
}
