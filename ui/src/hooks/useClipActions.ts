import { useState } from "react";
import type { ClipCard, CliptraceApi } from "../api";
import type { Clips } from "./useClips";
import type { Toasts } from "./useToasts";

/** Time Quick paste stays open after a copy so the "Copied" toast can be seen. */
const HIDE_AFTER_COPY_MS = 700;
/** How long a deleted clip can still be restored. */
const UNDO_WINDOW_MS = 5000;

type Deps = {
  api: CliptraceApi | null;
  clips: Clips;
  toasts: Toasts;
};

function messageOf(err: unknown, fallback: string) {
  return err instanceof Error ? err.message : fallback;
}

/** Everything that changes clips: copy, pin, tags, secret, delete and undo. */
export function useClipActions({ api, clips, toasts }: Deps) {
  const {
    cards,
    rawCards,
    current,
    pickIds,
    query,
    tab,
    setCards,
    setSelected,
    setPickIds,
    setAllTags,
    pendingDel,
    load,
    anchorSelection,
  } = clips;
  const [undoId, setUndoId] = useState<number | null>(null);
  const [secretPrompt, setSecretPrompt] = useState(false);

  /** Put the list back as it was and tell the user what failed. */
  function rollback(prev: ClipCard[], message: string) {
    setCards(prev);
    toasts.showError(message);
  }

  function replaceCard(next: ClipCard) {
    setCards((list) => list.map((c) => (c.id === next.id ? next : c)));
  }

  /** Run the staged deletes now (before the popup hides they must not be lost). */
  async function flushDeletes() {
    if (!api) return;
    for (const [id, handle] of pendingDel.current) {
      window.clearTimeout(handle);
      await api.deleteClip(id);
    }
    pendingDel.current.clear();
  }

  async function hideNow() {
    await flushDeletes();
    await api?.hidePopup();
  }

  async function copy(id?: number, hideAfter = false) {
    const card =
      id != null ? (cards.find((c) => c.id === id) ?? current) : current;
    if (!card) return;
    try {
      await api?.copyClip(card.id);
      toasts.flashCopied();
      if (hideAfter) {
        window.setTimeout(() => void hideNow(), HIDE_AFTER_COPY_MS);
      }
    } catch (err) {
      toasts.showError(messageOf(err, "Copy failed"), false);
    }
  }

  async function togglePin(card = current) {
    if (!card || !api) return;
    const prev = rawCards;
    anchorSelection();
    setCards((list) =>
      list.map((c) => (c.id === card.id ? { ...c, pinned: !c.pinned } : c)),
    );
    try {
      replaceCard(await api.setPinned(card.id, !card.pinned));
    } catch (err) {
      rollback(prev, messageOf(err, "Pin failed"));
    }
  }

  /** Pin every checked clip. If all are already pinned, unpin them instead. */
  function pinPicked() {
    if (!pickIds.length || !api) return;
    const picked = cards.filter((c) => pickIds.includes(c.id));
    const pin = !picked.every((c) => c.pinned);
    const prev = rawCards;
    anchorSelection();
    setCards((list) =>
      list.map((c) => (pickIds.includes(c.id) ? { ...c, pinned: pin } : c)),
    );
    void (async () => {
      try {
        for (const c of picked) {
          if (c.pinned !== pin) await api.setPinned(c.id, pin);
        }
        setPickIds([]);
        void load(true, query, tab, true);
      } catch (err) {
        rollback(prev, messageOf(err, "Pin failed"));
      }
    })();
  }

  async function addTag(tag: string) {
    if (!current || !api) return;
    const prev = rawCards;
    setCards((list) =>
      list.map((c) =>
        c.id === current.id ? { ...c, tags: [...c.tags, tag] } : c,
      ),
    );
    try {
      replaceCard(await api.addTag(current.id, tag));
      setAllTags(await api.listTagCounts());
    } catch (err) {
      rollback(prev, messageOf(err, "Tag failed"));
    }
  }

  async function removeTag(tag: string) {
    if (!current || !api) return;
    const prev = rawCards;
    setCards((list) =>
      list.map((c) =>
        c.id === current.id
          ? { ...c, tags: c.tags.filter((t) => t !== tag) }
          : c,
      ),
    );
    try {
      replaceCard(await api.removeTag(current.id, tag));
      setAllTags(await api.listTagCounts());
    } catch (err) {
      rollback(prev, messageOf(err, "Untag failed"));
    }
  }

  function askSecret() {
    if (!current || current.kind === "secret") return;
    setSecretPrompt(true);
  }

  async function confirmSecret() {
    if (!current || !api) return;
    const prev = rawCards;
    const id = current.id;
    setCards((list) =>
      list.map((c) =>
        c.id === id
          ? { ...c, kind: "secret", preview: "[secret …]", body: "[secret …]" }
          : c,
      ),
    );
    setSecretPrompt(false);
    try {
      replaceCard(await api.markClipSecret(id));
    } catch (err) {
      rollback(prev, messageOf(err, "Mark secret failed"));
    }
  }

  /** Hide the current clip now and delete it after the undo window. */
  function askDelete() {
    if (!current || !api) return;
    const saved = current;
    setCards((list) => list.filter((c) => c.id !== saved.id));
    setSelected((i) => Math.max(0, Math.min(i, cards.length - 2)));
    void api.stageDelete(saved.id);
    const handle = window.setTimeout(() => {
      pendingDel.current.delete(saved.id);
      setUndoId((id) => (id === saved.id ? null : id));
      void api.deleteClip(saved.id);
    }, UNDO_WINDOW_MS);
    pendingDel.current.set(saved.id, handle);
    setUndoId(saved.id);
    toasts.clearError();
  }

  function undoDelete() {
    if (undoId == null) return;
    const handle = pendingDel.current.get(undoId);
    if (handle != null) window.clearTimeout(handle);
    pendingDel.current.delete(undoId);
    void api?.unstageDelete(undoId);
    setUndoId(null);
    void load(true, query, tab, false);
  }

  /** Delete every checked clip at once (no undo). */
  function deletePicked() {
    if (!pickIds.length || !api) return;
    const ids = pickIds.slice();
    setCards((list) => list.filter((c) => !ids.includes(c.id)));
    setPickIds([]);
    setSelected(0);
    void (async () => {
      for (const id of ids) {
        await api.deleteClip(id);
      }
      setAllTags(await api.listTagCounts());
      void load(true);
    })();
  }

  return {
    undoId,
    secretPrompt,
    copy,
    hideNow,
    flushDeletes,
    togglePin,
    pinPicked,
    addTag,
    removeTag,
    askSecret,
    confirmSecret,
    cancelSecret: () => setSecretPrompt(false),
    askDelete,
    undoDelete,
    dismissUndo: () => setUndoId(null),
    deletePicked,
  };
}

export type ClipActions = ReturnType<typeof useClipActions>;
