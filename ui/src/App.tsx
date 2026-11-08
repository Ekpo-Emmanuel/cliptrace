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
import { SecretDialog } from "./components/SecretDialog";
import { SettingsPanel } from "./components/SettingsPanel";
import {
  applyTheme,
  loadTheme,
  useEffectiveTheme,
  type Theme,
} from "./lib/theme";
import { ErrorStrip, UndoStrip } from "./components/Strip";

type Tab = "recent" | "pinned";

export default function App() {
  const inApp = tauriAvailable();
  const apiRef = useRef<CliptraceApi | null>(null);
  const [query, setQuery] = useState("");
  const [tab, setTab] = useState<Tab>("recent");
  const [rawCards, setCards] = useState<ClipCard[]>([]);
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
  const [compact, setCompact] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [theme, setTheme] = useState<Theme>(loadTheme);
  useEffect(() => applyTheme(theme), [theme]);
  const shownTheme = useEffectiveTheme(theme);
  // Quick paste lists pinned clips first; selection follows this order.
  const cards = useMemo(
    () =>
      compact
        ? [...rawCards.filter((c) => c.pinned), ...rawCards.filter((c) => !c.pinned)]
        : rawCards,
    [rawCards, compact],
  );
  const remapId = useRef<number | null>(null);
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
      setSettingsOpen(false);
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
    const prev = rawCards;
    remapId.current = current?.id ?? null;
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
    const prev = rawCards;
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
    const prev = rawCards;
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
    const prev = rawCards;
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

  /** Pin every checked clip. If all are already pinned, unpin them instead. */
  function pinPicked() {
    const api = apiRef.current;
    if (!pickIds.length || !api) return;
    const picked = cards.filter((c) => pickIds.includes(c.id));
    const pin = !picked.every((c) => c.pinned);
    const prev = rawCards;
    remapId.current = current?.id ?? null;
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
        fail(prev, err instanceof Error ? err.message : "Pin failed");
      }
    })();
  }

  function moveSelection(to: number, extend = false) {
    if (!cards.length) return;
    const next = Math.max(0, Math.min(cards.length - 1, to));
    if (extend) {
      const ids = [cards[selected]?.id, cards[next]?.id].filter(
        (x): x is number => x != null,
      );
      setPickIds((cur) => Array.from(new Set([...cur, ...ids])));
    }
    setSelected(next);
    selectedId.current = cards[next]?.id ?? null;
    if (!done && next > cards.length - 8) void load(false);
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

  function toggleMode() {
    const next = !compact;
    remapId.current = current?.id ?? null;
    setCompact(next);
    void apiRef.current?.setMode(next);
    window.setTimeout(() => document.getElementById("q")?.focus(), 0);
  }

  // After a reorder (mode switch, pin toggle) keep the same clip selected.
  useEffect(() => {
    const id = remapId.current;
    if (id == null) return;
    remapId.current = null;
    const i = cards.findIndex((c) => c.id === id);
    if (i >= 0) setSelected(i);
  }, [cards]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      const tagFocus = document.activeElement?.id === "add-tag";
      if ((e.ctrlKey || e.metaKey) && e.key === ",") {
        e.preventDefault();
        setSettingsOpen((open) => !open);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        if (settingsOpen) {
          setSettingsOpen(false);
          document.getElementById("q")?.focus();
          return;
        }
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
      if (settingsOpen) return;
      if (confirmSecret) {
        if (e.key === "Enter") {
          e.preventDefault();
          void confirmMarkSecret();
        }
        return;
      }
      if (e.key === "Tab" && !tagFocus && suggest.length === 0) {
        e.preventDefault();
        toggleMode();
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
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        document.getElementById("q")?.focus();
        return;
      }
      if (
        (e.ctrlKey || e.metaKey) &&
        e.shiftKey &&
        e.key.toLowerCase() === "d"
      ) {
        e.preventDefault();
        setTheme(shownTheme === "dark" ? "light" : "dark");
        return;
      }
      if (tagFocus) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z" && undoId != null) {
        e.preventDefault();
        undoDelete();
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.code === "Space") {
        e.preventDefault();
        if (current) togglePick(current.id);
        return;
      }
      if (
        (e.ctrlKey || e.metaKey) &&
        e.key.toLowerCase() === "a" &&
        !query.trim()
      ) {
        e.preventDefault();
        setPickIds(cards.map((c) => c.id));
        return;
      }
      if (e.shiftKey && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
        e.preventDefault();
        moveSelection(selected + (e.key === "ArrowDown" ? 1 : -1), true);
        return;
      }
      if (e.key === "PageDown" || e.key === "PageUp") {
        e.preventDefault();
        moveSelection(selected + (e.key === "PageDown" ? 6 : -6));
        return;
      }
      if ((e.key === "Home" || e.key === "End") && !query) {
        e.preventDefault();
        moveSelection(e.key === "Home" ? 0 : cards.length - 1);
        return;
      }
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
        if (pickIds.length) pinPicked();
        else void togglePin();
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
        // With text after the caret, Del edits the query instead of a clip.
        const input = document.activeElement as HTMLInputElement;
        const atEnd =
          input.selectionStart === input.value.length &&
          input.selectionEnd === input.value.length;
        if (!pickIds.length && !atEnd) return;
        e.preventDefault();
        if (pickIds.length) deletePicked();
        else askDelete();
        return;
      }
      if (document.activeElement?.id !== "q") return;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        moveSelection(selected + 1);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        moveSelection(selected - 1);
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
    <div
      className={`relative flex flex-col overflow-hidden rounded-(--r-window) bg-(--bg) text-(--text) border border-[var(--line)] ${
        compact ? "tq h-[440px] w-[380px]" : "tr h-[580px] w-[880px]"
      }`}
    >
      <div
        className="flex h-3 shrink-0 cursor-grab items-center justify-center active:cursor-grabbing"
        aria-label="Move window"
        onMouseDown={(e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          void apiRef.current?.startDrag();
        }}
      >
        <span className="pointer-events-none h-0.5 w-8 rounded-full bg-(--line)" />
      </div>
      {note?.startsWith("keyword-only") ? (
        <div
          className="px-4 py-1 font-mono text-[10px]"
          style={{ color: "var(--faint)" }}
        >
          keyword-only search · embeddings unavailable
        </div>
      ) : null}
      <div className="relative">
        {pickIds.length > 0 ? (
          <PickBar
            count={pickIds.length}
            total={cards.length}
            onSelectAll={() => setPickIds(cards.map((c) => c.id))}
            onClear={() => setPickIds([])}
            onDelete={deletePicked}
          />
        ) : null}
        <SearchBar
          paused={paused}
          value={query}
          onChange={schedule}
          onClear={() => schedule("")}
          suggest={suggest}
          suggestIndex={suggestIndex}
          onPickSuggest={pickSuggest}
          compact={compact}
          onToggleMode={toggleMode}
          onSettings={() => setSettingsOpen(true)}
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
      <div className="relative mt-space-xs min-h-0 flex-1 overflow-hidden">
        {cards.length === 0 ? (
          <EmptyState
            kind={emptyKind}
            query={query}
            total={total}
            compact={compact}
            onClear={() => schedule("")}
            onBrowse={() => changeTab("recent")}
          />
        ) : (
          <div
            className={`grid h-full min-h-0 ${
              compact
                ? "grid-cols-[minmax(0,1fr)]"
                : "grid-cols-[minmax(0,344px)_minmax(0,1fr)]"
            }`}
          >
            <div className="flex min-h-0 flex-col">
              {paused ? (
                <div className="paused-band" role="status">
                  <span aria-hidden>❚❚</span>
                  <span>
                    <b style={{ color: "var(--text)" }}>Capture paused</b>
                    {" · copies aren’t saved"}
                  </span>
                </div>
              ) : null}
              <div className="min-h-0 flex-1">
                <ClipList
                  cards={cards}
                  selected={selected}
                  pickIds={pickIds}
                  onSelect={(index) => {
                    selectRow(index);
                    // A click copies the clip. Quick paste closes afterwards, like
                    // Enter; full view stays open so the preview can be read.
                    // While clips are checked for a bulk action, a click only
                    // moves the cursor.
                    const id = cards[index]?.id;
                    if (!pickIds.length && id != null) void copy(id, compact);
                  }}
                  onTogglePick={togglePick}
                  compact={compact}
                  onMore={() => {
                    if (!done) void load(false);
                  }}
                />
              </div>
            </div>
            {compact ? null : (
              <PreviewPane
                card={current}
                position={selected + 1}
                allTags={allTags}
                copied={copied}
                onCopy={() => void copy()}
                onPin={() => void togglePin()}
                onSecret={askSecret}
                onDelete={askDelete}
                onAddTag={(t) => void addTag(t)}
                onRemoveTag={(t) => void removeTag(t)}
              />
            )}
          </div>
        )}
      </div>
      {undoId != null ? (
        <UndoStrip
          key={undoId}
          onUndo={undoDelete}
          onDismiss={() => setUndoId(null)}
        />
      ) : null}
      {errToast ? (
        <ErrorStrip message={errToast} onDismiss={() => setErrToast(null)} />
      ) : null}
      <KeyHints
        empty={cards.length === 0}
        compact={compact}
        theme={shownTheme}
        onToggleTheme={() => setTheme(shownTheme === "dark" ? "light" : "dark")}
      />
      {settingsOpen ? (
        <SettingsPanel
          api={apiRef.current}
          theme={theme}
          onTheme={setTheme}
          onClose={() => {
            setSettingsOpen(false);
            window.setTimeout(() => document.getElementById("q")?.focus(), 0);
          }}
          onError={(message) => {
            setErrToast(message);
            window.setTimeout(() => setErrToast(null), 2400);
          }}
        />
      ) : null}
      {confirmSecret ? (
        <SecretDialog
          specimen={current?.body ?? ""}
          onCancel={() => setConfirmSecret(false)}
          onConfirm={() => void confirmMarkSecret()}
        />
      ) : null}
    </div>
  );

  if (inApp) return <div className="h-full">{frame}</div>;
  return (
    <div className="flex min-h-full items-center justify-center bg-(--panel) p-8">
      {frame}
    </div>
  );
}
