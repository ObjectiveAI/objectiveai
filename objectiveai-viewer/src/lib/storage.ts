import type { Entry } from "../types";

export interface StoredSession {
  id: string;
  startTime: number;
  endTime: number | null;
  entryCount: number;
  kinds: Entry["kind"][];
}

const DB_NAME = "objectiveai-viewer";
const DB_VERSION = 1;

let cachedDb: IDBDatabase | null = null;

function openDB(): Promise<IDBDatabase> {
  if (cachedDb) return Promise.resolve(cachedDb);
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains("sessions")) {
        const store = db.createObjectStore("sessions", { keyPath: "id" });
        store.createIndex("startTime", "startTime");
      }
      if (!db.objectStoreNames.contains("entries")) {
        const store = db.createObjectStore("entries", { autoIncrement: true });
        store.createIndex("sessionId", "sessionId");
      }
    };
    req.onsuccess = () => {
      cachedDb = req.result;
      cachedDb.onclose = () => { cachedDb = null; };
      resolve(cachedDb);
    };
    req.onerror = () => reject(req.error);
  });
}

function tx<T>(
  storeName: string | string[],
  mode: IDBTransactionMode,
  fn: (stores: Record<string, IDBObjectStore>) => IDBRequest | IDBRequest[],
): Promise<T> {
  return openDB().then((db) =>
    new Promise<T>((resolve, reject) => {
      const names = Array.isArray(storeName) ? storeName : [storeName];
      const transaction = db.transaction(names, mode);
      const stores: Record<string, IDBObjectStore> = {};
      for (const n of names) stores[n] = transaction.objectStore(n);
      const result = fn(stores);
      const req = Array.isArray(result) ? result[result.length - 1] : result;
      transaction.oncomplete = () => resolve(req.result as T);
      transaction.onerror = () => reject(transaction.error);
    }),
  );
}

export function saveSession(session: StoredSession): Promise<void> {
  return tx("sessions", "readwrite", (s) => s.sessions.put(session));
}

export function listSessions(): Promise<StoredSession[]> {
  return openDB().then((db) =>
    new Promise((resolve, reject) => {
      const transaction = db.transaction("sessions", "readonly");
      const store = transaction.objectStore("sessions");
      const index = store.index("startTime");
      const req = index.openCursor(null, "prev");
      const results: StoredSession[] = [];
      req.onsuccess = () => {
        const cursor = req.result;
        if (cursor) {
          results.push(cursor.value);
          cursor.continue();
        } else {
          resolve(results);
        }
      };
      req.onerror = () => reject(req.error);
    }),
  );
}

export function deleteSession(id: string): Promise<void> {
  return openDB().then((db) =>
    new Promise((resolve, reject) => {
      const transaction = db.transaction(["sessions", "entries"], "readwrite");
      transaction.objectStore("sessions").delete(id);
      const entryStore = transaction.objectStore("entries");
      const index = entryStore.index("sessionId");
      const req = index.openCursor(IDBKeyRange.only(id));
      req.onsuccess = () => {
        const cursor = req.result;
        if (cursor) {
          cursor.delete();
          cursor.continue();
        }
      };
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    }),
  );
}

export function putEntries(sessionId: string, entries: Entry[]): Promise<void> {
  return openDB().then((db) =>
    new Promise((resolve, reject) => {
      const transaction = db.transaction("entries", "readwrite");
      const store = transaction.objectStore("entries");
      const index = store.index("sessionId");
      const clearReq = index.openCursor(IDBKeyRange.only(sessionId));
      clearReq.onsuccess = () => {
        const cursor = clearReq.result;
        if (cursor) {
          cursor.delete();
          cursor.continue();
        } else {
          for (const entry of entries) {
            store.put({ sessionId, entryId: entry.id, entry });
          }
        }
      };
      transaction.oncomplete = () => resolve();
      transaction.onerror = () => reject(transaction.error);
    }),
  );
}

export function getEntries(sessionId: string): Promise<Entry[]> {
  return openDB().then((db) =>
    new Promise((resolve, reject) => {
      const transaction = db.transaction("entries", "readonly");
      const store = transaction.objectStore("entries");
      const index = store.index("sessionId");
      const req = index.getAll(IDBKeyRange.only(sessionId));
      req.onsuccess = () => {
        const rows = req.result as { entry: Entry }[];
        resolve(rows.map((r) => r.entry));
      };
      req.onerror = () => reject(req.error);
    }),
  );
}

export function pruneOldSessions(keepCount: number): Promise<void> {
  return listSessions().then((sessions) => {
    if (sessions.length <= keepCount) return;
    const toDelete = sessions.slice(keepCount);
    return Promise.all(toDelete.map((s) => deleteSession(s.id))).then(() => {});
  });
}
