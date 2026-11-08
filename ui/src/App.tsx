import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getApi,
  tauriAvailable,
  type BrowsePayload,
  type ClipCard,
  type CliptraceApi,
  type TagCount,
} from "./api";
import { ClipList } from "./components/ClipList";
import { PickBar } from "./components/PickBar";
import { EmptyState } from "./components/EmptyState";
import { KeyHints } from "./components/KeyHints";
import { PreviewPane } from "./components/PreviewPane";
import { SearchBar } from "./components/SearchBar";
import { Tabs } from "./components/Tabs";
import { IconShield } from "./icons";

type Tab = "recent" | "pinned";

export default function App() {
  const inApp = tauriAvailable();
  const apiRef = useRef<CliptraceApi | null>(null);
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("recent");
  const [cards, setCards] = useState<ClipCard[]>([]);
  const [selected, setSelected] = useState(0);
  const [pickIds, setPickIds] = useState<number[]>([]);
  const [offset, setOffset] = useState(0);
  const [done, setDone] = useState(false);
  const [pinned, setPinned] = useState(0);
  const [total, setTotal] = useState(0);
  const [note, setNote] = useState<string | null>(null);
  const [paused, setPaused] = useState(false);
  const [allTags, setAllTags] = useState<TagCount[]>([]);
  const [confirmSecret, setConfirmSecret] = useState(false);
  const [suggest, setSuggest] = useState<string[]>([]);
  const [suggestIndex, setSuggestIndex] = useState(0);
  const timer = useRef(0);
  const copiedTimer = useRef(0);
  const seq = useRef(0);
  const selectedId = useRef<number | null>(null);
  const pendingDel = useRef<Map<number, number>>(new Map());
  const queryRef = useRef(query);
  const tabRef = useRef(tab);
  const loadRef = useRef<
    (
      reset: boolean,
      q?: string,
      nextTab?: Tab,
      keepSelection?: boolean,
    ) => Promise<void>
  >(async () => {});
  const [copied, setCopied] = useState(false);
  const [errToast, setErrToast] = useState<string | null>(null);
  const [undoId, setUndoId] = useState<number | null>(null);
  const [fullText, setFullText] = useState<Record<number, string>>({});
  queryRef.current = query;
  tabRef.current = tab;

  const load = useCallback(
    async (
      reset: boolean,
      q = query,
      nextTab = tab,
      keepSelection = !reset,
    ) => {
      const api = apiRef.current ?? (await getApi());
      apiRef.current = api;
      const my = ++seq.current;
      const off = reset ? 0 : offset;
      const payload: BrowsePayload = await api.browseClips({
        query: q,
        tab: nextTab,
        tag: "",
        offset: off,
      });
      if (my !== seq.current) return;
      const hidden = new Set(pendingDel.current.keys());
      const next = payload.cards.filter((c) => !hidden.has(c.id));
      setCards((prev) => {
        const merged = reset ? next : prev.concat(next);
        if (keepSelection && selectedId.current != null) {
          const idx = merged.findIndex((c) => c.id === selectedId.current);
          setSelected(idx >= 0 ? idx : 0);
        } else if (reset && !keepSelection) {
          setSelected(0);
          selectedId.current = merged[0]?.id ?? null;
        }
        return merged;
      });
      if (reset && !keepSelection) setPickIds([]);
      setPinned(payload.pinned);
      setTotal(payload.total);
      const st = await api.status().catch(() => null);
      if (my !== seq.current) return;
      const keyword =
        st?.keyword_only ||
        (payload.note != null && payload.note.startsWith("keyword-only"));
      setNote(
        keyword
          ? (payload.note ?? "keyword-only: embeddings unavailable")
          : payload.note,
      );
      setPaused(st?.paused ?? payload.paused);
      setOffset(off + next.length);
      setDone(!!q.trim() || next.length < 50);
      const tags = await api.listTagCounts();
      if (my !== seq.current) return;
      setAllTags(tags);
    },
    [offset, query, tab],
  );

  useEffect(() => {
    loadRef.current = load;
  }, [load]);

  useEffect(() => {
    let unClips = () => {};
    let unStatus = () => {};
    void (async () => {
      const api = await getApi();
      apiRef.current = api;
      const st = await api.status();
      setPaused(st.paused);
      if (st.keyword_only) setNote("keyword-only: embeddings unavailable");
      await loadRef.current(true, "", "recent", false);
      unClips = await api.onClipsChanged(() => {
        void loadRef.current(true, queryRef.current, tabRef.current, true);
      });
      unStatus = await api.onStatusChanged((s) => {
        setPaused(s.paused);
        setNote((n) =>
          s.keyword_only
            ? n?.startsWith("keyword-only")
              ? n
              : "keyword-only: embeddings unavailable"
            : n?.startsWith("keyword-only")
              ? null
              : n,
        );
      });
    })();
    window.cliptraceOnOpen = (cache) => {
      setQuery("");
      setTab("recent");
      setPickIds([]);
      if (cache?.cards?.length) {
        setCards(cache.cards);
        setSelected(0);
        selectedId.current = cache.cards[0]?.id ?? null;
        setPaused(!!cache.paused);
        if (cache.keyword_only) {
          setNote("keyword-only: embeddings unavailable");
        }
      }
      void loadRef.current(true, "", "recent", false);
      document.getElementById("q")?.focus();
    };
    window.cliptraceOnHide = () => {
      void flushDeletes();
    };
    return () => {
      unClips();
      unStatus();
    };
  }, []);

  useEffect(() => {
    const card = cards[selected];
    if (!card || card.kind === "secret") return;
    if (fullText[card.id]) return;
    void apiRef.current
      ?.clipText(card.id)
      .then((text) => {
        setFullText((map) => ({ ...map, [card.id]: text }));
      })
      .catch(() => {});
  }, [cards, selected, fullText]);

  function schedule(value: string) {
    setQuery(value);
    const hash = value.match(/(?:^|\s)#([^\s]*)$/);
    if (hash) {
      const prefix = hash[1].toLowerCase();
      const names = allTags
        .filter((t) => t.name.toLowerCase().includes(prefix))
        .map((t) => t.name);
      setSuggest(names.slice(0, 8));
      setSuggestIndex(0);
    } else {
      setSuggest([]);
    }
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      void load(true, value, tab);
    }, 150);
  }

  function pickSuggest(name: string) {
    const next = query.replace(
      /(?:^|\s)#([^\s]*)$/,
      (all) => `${all.startsWith(" ") ? " " : ""}#${name} `,
    );
    setSuggest([]);
    schedule(next);
    document.getElementById("q")?.focus();
  }

  const current = cards[selected]
    ? {
        ...cards[selected],
        body:
          cards[selected].kind === "secret"
            ? cards[selected].preview
            : (fullText[cards[selected].id] ?? cards[selected].preview),
      }
    : undefined;

  function flashCopied() {
    setCopied(true);
    window.clearTimeout(copiedTimer.current);
    copiedTimer.current = window.setTimeout(() => setCopied(false), 2000);
  }

  async function copy(id?: number, hideAfter = false) {
    const card =
      id != null ? (cards.find((c) => c.id === id) ?? current) : current;
    if (!card) return;
    try {
      await apiRef.current?.copyClip(card.id);
      flashCopied();
      if (hideAfter) {
        window.setTimeout(() => void hideNow(), 280);
      }
    } catch (err) {
      setErrToast(err instanceof Error ? err.message : "Copy failed");
    }
  }

  async function hideNow() {
    await flushDeletes();
    await apiRef.current?.hidePopup();
  }

  async function flushDeletes() {
    const api = apiRef.current;
    if (!api) return;
    for (const [id, handle] of pendingDel.current) {
      window.clearTimeout(handle);
      await api.deleteClip(id);
    }
    pendingDel.current.clear();
  }

  function selectRow(index: number) {
    setSelected(index);
    selectedId.current = cards[index]?.id ?? null;
  }

  function fail(prev: ClipCard[], message: string) {
    setCards(prev);
    setErrToast(message);
    window.setTimeout(() => setErrToast(null), 2400);
  }

  async function togglePin(card = current) {
    if (!card || !apiRef.current) return;
    const prev = cards;
    setCards((list) =>
      list.map((c) => (c.id === card.id ? { ...c, pinned: !c.pinned } : c)),
    );
    try {
      const next = await apiRef.current.setPinned(card.id, !card.pinned);
      setCards((list) => list.map((c) => (c.id === next.id ? next : c)));
    } catch (err) {
      fail(prev, err instanceof Error ? err.message : "Pin failed");
    }
  }

  async function addTag(tag: string) {
    if (!current || !apiRef.current) return;
    const prev = cards;
    setCards((list) =>
      list.map((c) =>
        c.id === current.id ? { ...c, tags: [...c.tags, tag] } : c,
      ),
    );
    try {
      const next = await apiRef.current.addTag(current.id, tag);
      setCards((list) => list.map((c) => (c.id === next.id ? next : c)));
      setAllTags(await apiRef.current.listTagCounts());
    } catch (err) {
      fail(prev, err instanceof Error ? err.message : "Tag failed");
    }
  }

  async function removeTag(tag: string) {
    if (!current || !apiRef.current) return;
    const prev = cards;
    setCards((list) =>
      list.map((c) =>
        c.id === current.id
          ? { ...c, tags: c.tags.filter((t) => t !== tag) }
          : c,
      ),
    );
    try {
      const next = await apiRef.current.removeTag(current.id, tag);
      setCards((list) => list.map((c) => (c.id === next.id ? next : c)));
      setAllTags(await apiRef.current.listTagCounts());
    } catch (err) {
      fail(prev, err instanceof Error ? err.message : "Untag failed");
    }
  }

  function askSecret() {
    if (!current || current.kind === "secret") return;
    setConfirmSecret(true);
  }

  async function confirmMarkSecret() {
    if (!current || !apiRef.current) return;
    const prev = cards;
    const id = current.id;
    setCards((list) =>
      list.map((c) =>
        c.id === id
          ? { ...c, kind: "secret", preview: "[secret …]", body: "[secret …]" }
          : c,
      ),
    );
    setConfirmSecret(false);
    try {
      const next = await apiRef.current.markClipSecret(id);
      setCards((list) => list.map((c) => (c.id === next.id ? next : c)));
    } catch (err) {
      fail(prev, err instanceof Error ? err.message : "Mark secret failed");
    }
  }

  function togglePick(id: number) {
    setPickIds((ids) =>
      ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id],
    );
  }

  function deletePicked() {
    if (!pickIds.length || !apiRef.current) return;
    const ids = pickIds.slice();
    setCards((list) => list.filter((c) => !ids.includes(c.id)));
    setPickIds([]);
    setSelected(0);
    void (async () => {
      for (const id of ids) {
        await apiRef.current?.deleteClip(id);
      }
      const tags = await apiRef.current?.listTagCounts();
      if (tags) setAllTags(tags);
      void load(true);
    })();
  }

  function askDelete() {
    if (!current || !apiRef.current) return;
    const saved = current;
    setCards((list) => list.filter((c) => c.id !== saved.id));
    setSelected((i) => Math.max(0, Math.min(i, cards.length - 2)));
    void apiRef.current.stageDelete(saved.id);
    const handle = window.setTimeout(() => {
      pendingDel.current.delete(saved.id);
      setUndoId((id) => (id === saved.id ? null : id));
      void apiRef.current?.deleteClip(saved.id);
    }, 5000);
    pendingDel.current.set(saved.id, handle);
    setUndoId(saved.id);
    setErrToast(null);
  }

  function undoDelete() {
    if (undoId == null) return;
    const handle = pendingDel.current.get(undoId);
    if (handle != null) window.clearTimeout(handle);
    pendingDel.current.delete(undoId);
    void apiRef.current?.unstageDelete(undoId);
    setUndoId(null);
    void load(true, query, tab, false);
  }

  function changeTab(next: Tab) {
    setTab(next);
    void load(true, query, next);
  }

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const tagFocus = document.activeElement?.id === "add-tag";
      if (e.key === "Escape") {
        e.preventDefault();
        if (pickIds.length) {
          setPickIds([]);
          return;
        }
        if (confirmSecret) {
          setConfirmSecret(false);
          return;
        }
        if (suggest.length) {
          setSuggest([]);
          return;
        }
        void hideNow();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "1") {
        e.preventDefault();
        changeTab("recent");
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "2") {
        e.preventDefault();
        changeTab("pinned");
        return;
      }
      if (tagFocus) return;
      if (suggest.length && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
        e.preventDefault();
        setSuggestIndex((i) =>
          Math.max(
            0,
            Math.min(suggest.length - 1, i + (e.key === "ArrowDown" ? 1 : -1)),
          ),
        );
        return;
      }
      if (suggest.length && e.key === "Enter") {
        e.preventDefault();
        const name = suggest[suggestIndex];
        if (name) pickSuggest(name);
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "p") {
        e.preventDefault();
        void togglePin();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "t") {
        e.preventDefault();
        document.getElementById("add-tag")?.focus();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "l") {
        e.preventDefault();
        askSecret();
        return;
      }
      if (e.key === "Delete" && document.activeElement?.id === "q") {
        e.preventDefault();
        askDelete();
        return;
      }
      if (document.activeElement?.id !== "q") return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((i) => {
          const n = Math.min(cards.length - 1, i + 1);
          if (n > cards.length - 8) void load(false);
          return Math.max(0, n);
        });
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((i) => Math.max(0, i - 1));
      } else if (e.key === "Enter") {
        e.preventDefault();
        void copy(undefined, true);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const emptyKind = useMemo(() => {
    if (query.trim()) return "results" as const;
    if (tab === "pinned") return "pins" as const;
    return "history" as const;
  }, [query, tab]);

  const frame = (
    <div className="relative flex h-[500px] w-[680px] flex-col overflow-hidden rounded-lg bg-white">
      <div
        className="flex h-3 shrink-0 cursor-grab items-center justify-center bg-white active:cursor-grabbing"
        aria-label="Move window"
        onMouseDown={(e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          void apiRef.current?.startDrag();
        }}
      >
        <span className="pointer-events-none h-0.5 w-8 rounded-full bg-outline-variant/50" />
      </div>
      {paused ? (
        <div className="font-label-md flex items-center gap-2 bg-tertiary-container px-space-md py-1 text-on-tertiary-container">
          <IconShield className="h-3.5 w-3.5" />
          Capture paused
        </div>
      ) : null}
      {note?.startsWith("keyword-only") ? (
        <div className="font-label-md flex items-center gap-2 bg-secondary-container px-space-md py-1 text-on-secondary-container">
          Keyword-only search
        </div>
      ) : null}
      <div className="px-space-md pt-space-sm">
        <SearchBar
          value={query}
          onChange={schedule}
          onClear={() => schedule("")}
          suggest={suggest}
          suggestIndex={suggestIndex}
          onPickSuggest={pickSuggest}
          trailing={
            <Tabs
              tab={tab}
              total={total}
              pinned={pinned}
              onChange={changeTab}
            />
          }
        />
      </div>
      <div className="relative mt-space-xs min-h-0 flex-1 overflow-hidden bg-white">
        {cards.length === 0 ? (
          <EmptyState kind={emptyKind} query={query} onExample={schedule} />
        ) : (
          <div className="grid h-full min-h-0 grid-cols-[minmax(0,45%)_minmax(0,55%)] bg-white">
            <div className="flex min-h-0 flex-col">
              {pickIds.length > 0 ? (
                <PickBar
                  count={pickIds.length}
                  total={cards.length}
                  onSelectAll={() => setPickIds(cards.map((c) => c.id))}
                  onClear={() => setPickIds([])}
                  onDelete={deletePicked}
                />
              ) : null}
              <div className="min-h-0 flex-1">
                <ClipList
                  cards={cards}
                  selected={selected}
                  pickIds={pickIds}
                  onSelect={selectRow}
                  onTogglePick={togglePick}
                  onPin={(card) => void togglePin(card)}
                  onTag={(i) => {
                    setSelected(i);
                    queueMicrotask(() =>
                      document.getElementById("add-tag")?.focus(),
                    );
                  }}
                  onMore={() => {
                    if (!done) void load(false);
                  }}
                />
              </div>
            </div>
            <PreviewPane
              card={current}
              allTags={allTags}
              copied={copied}
              onCopy={() => void copy()}
              onPin={() => void togglePin()}
              onSecret={askSecret}
              onDelete={askDelete}
              onAddTag={(t) => void addTag(t)}
              onRemoveTag={(t) => void removeTag(t)}
            />
          </div>
        )}
      </div>
      {undoId != null ? (
        <div className="font-label-xs flex items-center gap-2 border-t border-outline-variant/30 bg-surface-container-low px-2.5 py-1.5 text-on-surface">
          <span>Deleted</span>
          <button
            type="button"
            className="font-label-xs ml-auto cursor-pointer text-primary hover:underline"
            onClick={undoDelete}
          >
            Undo
          </button>
        </div>
      ) : null}
      {errToast ? (
        <div className="font-label-xs border-t border-outline-variant/30 bg-error-container px-2.5 py-1.5 text-on-error-container">
          {errToast}
        </div>
      ) : null}
      <KeyHints empty={cards.length === 0} />
      {confirmSecret ? (
        <div className="absolute inset-0 z-30 flex items-end justify-center bg-slate/40 p-space-md">
          <div className="w-full max-w-sm rounded-md bg-surface-container-lowest p-space-md">
            <p className="font-body-sm text-on-surface">
              Mark as secret is one-way. Plaintext is encrypted and removed from
              the database.
            </p>
            <div className="mt-space-sm flex justify-end gap-space-xs">
              <button
                type="button"
                className="font-label-md rounded-md px-space-sm py-space-xs text-on-surface cursor-pointer"
                onClick={() => setConfirmSecret(false)}
              >
                Cancel
              </button>
              <button
                type="button"
                className="font-label-md rounded bg-accent px-space-sm  text-on-primary cursor-pointer"
                onClick={() => void confirmMarkSecret()}
              >
                Mark as secret
              </button>
            </div>
          </div>
        </div>
      ) : null}
    </div>
  );

  if (inApp) return <div className="h-full bg-white">{frame}</div>;
  return (
    <div className="flex min-h-full items-center justify-center bg-surface-dim p-8">
      {frame}
    </div>
  );
}
