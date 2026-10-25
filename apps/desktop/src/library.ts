// The private library of the open account: what is saved, and saving one document after another
// (each copy is fetched from its publisher at the server's pace), so the work goes on while the
// user moves between screens.

import { useCallback, useMemo, useRef, useState } from "react";

import { api, type PrivateItem, type RefusedReason } from "./api";

/** The progress, and then the result, of saving a list. */
export interface Saving {
  total: number;
  done: number;
  saved: number;
  already: number;
  refused: number;
  running: boolean;
  /** Why documents were not saved, by document ID. */
  reasons: Map<number, RefusedReason>;
  /** What stopped the saving (the part was locked, ATENA closing…). */
  error: unknown;
}

export interface Library {
  items: PrivateItem[];
  /** Saved documents of the public part, by document ID. */
  byDocument: Map<number, PrivateItem>;
  saving: Saving | null;
  refresh: () => void;
  /** Saves the documents one by one; ignored while another list is being saved. */
  save: (documentIds: number[]) => void;
  stop: () => void;
  dismiss: () => void;
  remove: (ids: string[]) => Promise<void>;
  /** Forgets everything shown (the part was locked). */
  clear: () => void;
}

export function useLibrary(): Library {
  const [items, setItems] = useState<PrivateItem[]>([]);
  const [saving, setSaving] = useState<Saving | null>(null);
  const running = useRef(false);
  const stopping = useRef(false);
  // a new generation after `clear`: a list still being saved no longer reports
  const generation = useRef(0);

  const refresh = useCallback(() => {
    api.privateLibrary().then(setItems, () => setItems([]));
  }, []);

  const save = useCallback((documentIds: number[]) => {
    if (running.current || documentIds.length === 0) return;
    running.current = true;
    stopping.current = false;
    const mine = generation.current;
    const state: Saving = {
      total: documentIds.length,
      done: 0,
      saved: 0,
      already: 0,
      refused: 0,
      running: true,
      reasons: new Map(),
      error: null,
    };
    setSaving({ ...state });
    void (async () => {
      for (const id of documentIds) {
        if (stopping.current || generation.current !== mine) break;
        try {
          const answer = await api.privateSave(id);
          if (answer.status === "saved" && answer.item) {
            state.saved += 1;
            const item = answer.item;
            setItems((before) => [item, ...before.filter((i) => i.id !== item.id)]);
          } else if (answer.status === "already") {
            state.already += 1;
          } else {
            state.refused += 1;
            state.reasons.set(id, answer.reason ?? "unavailable");
          }
        } catch (e) {
          state.error = e;
          break;
        }
        state.done += 1;
        if (generation.current === mine) setSaving({ ...state, reasons: new Map(state.reasons) });
      }
      running.current = false;
      if (generation.current === mine) setSaving({ ...state, running: false, reasons: new Map(state.reasons) });
    })();
  }, []);

  const remove = useCallback(async (ids: string[]) => {
    await api.privateRemove(ids);
    setItems((before) => before.filter((i) => !ids.includes(i.id)));
  }, []);

  const stop = useCallback(() => {
    stopping.current = true;
  }, []);
  const dismiss = useCallback(() => {
    if (!running.current) setSaving(null);
  }, []);
  const clear = useCallback(() => {
    generation.current += 1;
    setItems([]);
    setSaving(null);
  }, []);

  const byDocument = useMemo(() => {
    const map = new Map<number, PrivateItem>();
    for (const item of items) {
      if (item.documentId !== null) map.set(item.documentId, item);
    }
    return map;
  }, [items]);

  return {
    items,
    byDocument,
    saving,
    refresh,
    save,
    stop,
    dismiss,
    remove,
    clear,
  };
}
