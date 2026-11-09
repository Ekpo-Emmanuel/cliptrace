import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  getApi,
  type BrowsePayload,
  type ClipCard,
  type CliptraceApi,
  type TagCount,
} from "../api";

export type Tab = "recent" | "pinned";
export type EmptyKind = "results" | "pins" | "history";
export type OpenCache = Parameters<NonNullable<Window["cliptraceOnOpen"]>>[0];

const PAGE_SIZE = 50;
export const KEYWORD_ONLY_NOTE = "keyword-only: embeddings unavailable";

/** Keep or drop the keyword-only note when the backend reports a status change. */
function noteForStatus(note: string | null, keywordOnly: boolean) {
  const isKeywordNote = note?.startsWith("keyword-only");
  if (keywordOnly) return isKeywordNote ? note : KEYWORD_ONLY_NOTE;
  return isKeywordNote ? null : note;
}

/**
 * The clip list: what is loaded, which tab and query it came from, the cursor, and
 * the checked clips. Mutations (pin, tag, delete...) live in `useClipActions`.
 * In Quick paste (`compact`) pinned clips are listed first and the cursor follows
 * that order.
 */
export function useClips(api: CliptraceApi | null, compact: boolean) {
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
  const [fullText, setFullText] = useState<Record<number, string>>({});

  const seq = useRef(0);
  const selectedId = useRef<number | null>(null);
  /** Clips staged for deletion (undo window), mapped to their pending timer. */
  const pendingDel = useRef<Map<number, number>>(new Map());
  /** Set before a reorder so the same clip stays selected afterwards. */
  const remapId = useRef<number | null>(null);
  const queryRef = useRef(query);
  const tabRef = useRef(tab);
  queryRef.current = query;
  tabRef.current = tab;

  const cards = useMemo(
    () =>
      compact
        ? [...rawCards.filter((c) => c.pinned), ...rawCards.filter((c) => !c.pinned)]
        : rawCards,
    [rawCards, compact],
  );

  const current = cards[selected]
    ? {
        ...cards[selected],
        body:
          cards[selected].kind === "secret"
            ? cards[selected].preview
            : (fullText[cards[selected].id] ?? cards[selected].preview),
      }
    : undefined;

  const emptyKind: EmptyKind = query.trim()
    ? "results"
    : tab === "pinned"
      ? "pins"
      : "history";

  const load = useCallback(
    async (
      reset: boolean,
      q = query,
      nextTab = tab,
      keepSelection = !reset,
    ) => {
      const client = await getApi();
      const my = ++seq.current;
      const off = reset ? 0 : offset;
      const payload: BrowsePayload = await client.browseClips({
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
      const st = await client.status().catch(() => null);
      if (my !== seq.current) return;
      const keyword =
        st?.keyword_only ||
        (payload.note != null && payload.note.startsWith("keyword-only"));
      setNote(
        keyword ? (payload.note ?? KEYWORD_ONLY_NOTE) : payload.note,
      );
      setPaused(st?.paused ?? payload.paused);
      setOffset(off + next.length);
      setDone(!!q.trim() || next.length < PAGE_SIZE);
      const tags = await client.listTagCounts();
      if (my !== seq.current) return;
      setAllTags(tags);
    },
    [offset, query, tab],
  );

  // Event listeners and first load run once; they call the latest `load`.
  const loadRef = useRef(load);
  useEffect(() => {
    loadRef.current = load;
  }, [load]);

  useEffect(() => {
    if (!api) return;
    let live = true;
    const unlisten: Array<() => void> = [];
    void (async () => {
      const st = await api.status();
      if (!live) return;
      setPaused(st.paused);
      if (st.keyword_only) setNote(KEYWORD_ONLY_NOTE);
      await loadRef.current(true, "", "recent", false);
      const unClips = await api.onClipsChanged(() => {
        void loadRef.current(true, queryRef.current, tabRef.current, true);
      });
      if (!live) return unClips();
      unlisten.push(unClips);
      const unStatus = await api.onStatusChanged((s) => {
        setPaused(s.paused);
        setNote((n) => noteForStatus(n, s.keyword_only));
      });
      if (!live) return unStatus();
      unlisten.push(unStatus);
    })();
    return () => {
      live = false;
      unlisten.forEach((off) => off());
    };
  }, [api]);

  // Fetch the full text of the selected clip (rows only carry a preview).
  useEffect(() => {
    const card = cards[selected];
    if (!card || card.kind === "secret") return;
    if (fullText[card.id]) return;
    void api
      ?.clipText(card.id)
      .then((text) => {
        setFullText((map) => ({ ...map, [card.id]: text }));
      })
      .catch(() => {});
  }, [api, cards, selected, fullText]);

  // After a reorder (mode switch, pin toggle) keep the same clip selected.
  useEffect(() => {
    const id = remapId.current;
    if (id == null) return;
    remapId.current = null;
    const i = cards.findIndex((c) => c.id === id);
    if (i >= 0) setSelected(i);
  }, [cards]);

  /** Call right before something reorders the list, e.g. a pin or a mode switch. */
  const anchorSelection = useCallback(() => {
    remapId.current = current?.id ?? null;
  }, [current?.id]);

  const selectRow = (index: number) => {
    setSelected(index);
    selectedId.current = cards[index]?.id ?? null;
  };

  /** Move the cursor. With `extend`, also check every clip it passes over. */
  const moveSelection = (to: number, extend = false) => {
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
  };

  const loadMore = () => {
    if (!done) void load(false);
  };

  const changeTab = (next: Tab) => {
    setTab(next);
    void load(true, query, next);
  };

  const togglePick = (id: number) =>
    setPickIds((ids) =>
      ids.includes(id) ? ids.filter((x) => x !== id) : [...ids, id],
    );
  const selectAll = () => setPickIds(cards.map((c) => c.id));
  const clearPicks = () => setPickIds([]);

  /** The popup was just shown: start from a clean recent list. */
  const resetForOpen = useCallback((cache?: OpenCache) => {
    setQuery("");
    setTab("recent");
    setPickIds([]);
    if (cache?.cards?.length) {
      setCards(cache.cards);
      setSelected(0);
      selectedId.current = cache.cards[0]?.id ?? null;
      setPaused(!!cache.paused);
      if (cache.keyword_only) setNote(KEYWORD_ONLY_NOTE);
    }
    void loadRef.current(true, "", "recent", false);
    document.getElementById("q")?.focus();
  }, []);

  return {
    // data
    cards,
    rawCards,
    current,
    selected,
    pickIds,
    pinned,
    total,
    note,
    paused,
    allTags,
    query,
    tab,
    done,
    emptyKind,
    // loading
    load,
    loadMore,
    changeTab,
    resetForOpen,
    // cursor and checks
    selectRow,
    moveSelection,
    togglePick,
    selectAll,
    clearPicks,
    anchorSelection,
    // for the mutation and search hooks
    setQuery,
    setCards,
    setSelected,
    setPickIds,
    setAllTags,
    pendingDel,
  };
}

export type Clips = ReturnType<typeof useClips>;
