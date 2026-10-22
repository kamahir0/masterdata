import type { Directory } from "./filesystem";

const DB_NAME = "masterdata-browser";
const DB_VERSION = 1;
const STORE_NAME = "handles";
const RECENT_KEY = "recent-root";

function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    if (typeof indexedDB === "undefined") {
      reject(new Error("IndexedDB is not supported"));
      return;
    }
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function transaction<T>(
  mode: IDBTransactionMode,
  operation: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  const db = await openDB();
  try {
    return await new Promise<T>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, mode);
      let value: T;
      tx.oncomplete = () => resolve(value);
      tx.onerror = () => reject(tx.error);
      tx.onabort = () => reject(tx.error ?? new Error("IndexedDB transaction aborted"));
      const request = operation(tx.objectStore(STORE_NAME));
      request.onsuccess = () => { value = request.result; };
      request.onerror = () => reject(request.error);
    });
  } finally {
    db.close();
  }
}

export async function saveRecentDirectory(handle: Directory): Promise<void> {
  try {
    await transaction("readwrite", (store) => store.put(handle, RECENT_KEY));
  } catch {
    // IndexedDB failure should not block project usage.
  }
}

export async function getRecentDirectory(): Promise<Directory | null> {
  try {
    return (await transaction("readonly", (store) => store.get(RECENT_KEY))) ?? null;
  } catch {
    return null;
  }
}

export async function clearRecentDirectory(): Promise<void> {
  try {
    await transaction("readwrite", (store) => store.delete(RECENT_KEY));
  } catch {
    // Ignore cleanup errors
  }
}
