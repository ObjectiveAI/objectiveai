import { useEffect, useState } from "react";
import { dbQueryExecute } from "@objectiveai/sdk";
import { websocketExecutor } from "../lib/websocket-executor";

/** The most recent assistant text the agent wrote — the newest
 * `assistant_response_content_text` row in its history. */
const LATEST_TEXT_SQL = (hierarchy: string) => {
  const aih = hierarchy.replace(/'/g, "''");
  return (
    "SELECT t.text FROM objectiveai.messages m " +
    "JOIN objectiveai.assistant_response_content_text t " +
    "ON t.response_id = m.response_id " +
    'AND t."index" = m.row_index ' +
    "AND t.part_index = m.row_sub_index " +
    `WHERE m.agent_instance_hierarchy = '${aih}' ` +
    "AND m.\"table\" = 'assistant_response_content_text' " +
    'ORDER BY m."index" DESC LIMIT 1'
  );
};

/** How often the text refreshes while the agent is active. */
const ACTIVE_POLL_MS = 60_000;

/**
 * The most recent assistant TEXT content one agent wrote, via a
 * custom `db query` (newest `assistant_response_content_text` row in
 * the agent's history). `null` when the agent has written none yet
 * (or while the first read is in flight).
 *
 * Refresh cadence: one read whenever `hierarchy` or `active`
 * changes — which covers mount AND the active→inactive flip (the
 * final text lands right as the agent stops) — plus a 60-second poll
 * while the agent is active.
 */
export function useAgentLatestText(
  hierarchy: string,
  active: boolean,
): string | null {
  const [text, setText] = useState<string | null>(null);

  // A new hierarchy starts from scratch; active flips keep the last
  // text on screen while the refresh races.
  useEffect(() => {
    setText(null);
  }, [hierarchy]);

  useEffect(() => {
    let cancelled = false;

    const fetchText = async () => {
      try {
        const executor = await websocketExecutor();
        const response = await dbQueryExecute(executor, {
          query: LATEST_TEXT_SQL(hierarchy),
        });
        if (cancelled) return;
        // In-band CliError — keep whatever we had.
        if ("type" in response) return;
        const cell = response.rows[0]?.[0];
        setText(typeof cell === "string" ? cell : null);
      } catch {
        // Daemon unreachable — keep whatever we had.
      }
    };

    // Runs on mount and on EVERY active flip — including
    // active→inactive, which is exactly when the final text lands.
    void fetchText();
    const interval = active
      ? setInterval(() => {
          void fetchText();
        }, ACTIVE_POLL_MS)
      : null;

    return () => {
      cancelled = true;
      if (interval !== null) {
        clearInterval(interval);
      }
    };
  }, [hierarchy, active]);

  return text;
}
