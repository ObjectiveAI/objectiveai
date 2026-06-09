import { useState, useEffect, useRef, useCallback } from "react";
import type { Entry } from "../types";
import {
  saveSession,
  listSessions,
  deleteSession as deleteStoredSession,
  renameSession as renameStoredSession,
  putEntries,
  getEntries,
  pruneOldSessions,
  type StoredSession,
} from "../lib/storage";

const DEBOUNCE_MS = 2000;
const MAX_SESSIONS = 20;
const RESTORE_WINDOW_MS = 60 * 60 * 1000;

export function useSessionStorage(entries: Entry[], isLive: boolean) {
  const [sessionId] = useState(() => crypto.randomUUID());
  const [pastSessions, setPastSessions] = useState<StoredSession[]>([]);
  const [restoredEntries, setRestoredEntries] = useState<Entry[] | null>(null);
  const [restoredTimestamp, setRestoredTimestamp] = useState<number>(0);
  const [showRestoreBanner, setShowRestoreBanner] = useState(false);
  const [viewingSessionId, setViewingSessionId] = useState<string | null>(null);

  const entriesRef = useRef(entries);
  entriesRef.current = entries;
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const sessionStartRef = useRef(Date.now());

  useEffect(() => {
    listSessions().then((sessions) => {
      setPastSessions(sessions);
      if (sessions.length > 0) {
        const latest = sessions[0];
        const age = Date.now() - (latest.endTime ?? latest.startTime);
        if (age < RESTORE_WINDOW_MS && latest.entryCount > 0) {
          getEntries(latest.id).then((restored) => {
            if (restored.length > 0) {
              setRestoredEntries(restored);
              setRestoredTimestamp(latest.endTime ?? latest.startTime);
              setShowRestoreBanner(true);
            }
          });
        }
      }
    });
    pruneOldSessions(MAX_SESSIONS);
  }, []);

  useEffect(() => {
    if (!isLive || entries.length === 0) return;

    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      const current = entriesRef.current;
      const kinds = [...new Set(current.map((e) => e.kind))];
      putEntries(sessionId, current);
      saveSession({
        id: sessionId,
        startTime: sessionStartRef.current,
        endTime: Date.now(),
        entryCount: current.length,
        kinds,
      });
    }, DEBOUNCE_MS);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [entries, isLive, sessionId]);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
      const current = entriesRef.current;
      if (current.length > 0) {
        const kinds = [...new Set(current.map((e) => e.kind))];
        putEntries(sessionId, current);
        saveSession({
          id: sessionId,
          startTime: sessionStartRef.current,
          endTime: Date.now(),
          entryCount: current.length,
          kinds,
        });
      }
    };
  }, [sessionId]);

  const dismissRestore = useCallback(() => {
    setShowRestoreBanner(false);
    setRestoredEntries(null);
  }, []);

  const loadSession = useCallback(async (id: string) => {
    const loaded = await getEntries(id);
    setRestoredEntries(loaded);
    setViewingSessionId(id);
    setShowRestoreBanner(false);
  }, []);

  const returnToLive = useCallback(() => {
    setRestoredEntries(null);
    setViewingSessionId(null);
  }, []);

  const deleteSession = useCallback(async (id: string) => {
    await deleteStoredSession(id);
    setPastSessions((prev) => prev.filter((s) => s.id !== id));
    if (viewingSessionId === id) returnToLive();
  }, [viewingSessionId, returnToLive]);

  const renameSession = useCallback(async (id: string, name: string) => {
    await renameStoredSession(id, name);
    const sessions = await listSessions();
    setPastSessions(sessions);
  }, []);

  const exportSession = useCallback(async (id: string) => {
    const entries = await getEntries(id);
    const sessionMeta = pastSessions.find((s) => s.id === id);
    if (!sessionMeta) return;
    const payload = { session: sessionMeta, entries };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const date = new Date(sessionMeta.startTime);
    const filename = `objectiveai-session-${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}-${String(date.getHours()).padStart(2, "0")}${String(date.getMinutes()).padStart(2, "0")}.json`;
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    a.style.display = "none";
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, [pastSessions]);

  const refreshSessions = useCallback(async () => {
    const sessions = await listSessions();
    setPastSessions(sessions);
  }, []);

  return {
    sessionId,
    restoredEntries,
    restoredTimestamp,
    showRestoreBanner,
    dismissRestore,
    pastSessions,
    loadSession,
    deleteSession,
    renameSession,
    exportSession,
    isViewingPast: viewingSessionId !== null,
    returnToLive,
    refreshSessions,
  };
}
