const API_BASE =
  process.env.NEXT_PUBLIC_API_BASE ?? "https://api.objectiveai.dev";

/** Fetch JSON from the ObjectiveAI API */
export async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`API ${path}: ${res.status}`);
  return res.json();
}
