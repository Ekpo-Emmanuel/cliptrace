import { useRef, useState } from "react";
import type { Clips } from "./useClips";

const SEARCH_DEBOUNCE_MS = 150;
const MAX_SUGGESTIONS = 8;
/** A `#tag` the caret is still typing at the end of the query. */
const TRAILING_TAG = /(?:^|\s)#([^\s]*)$/;

/** The search box: debounced querying plus `#tag` suggestions while typing. */
export function useClipSearch(clips: Clips) {
  const { query, tab, allTags, load, setQuery } = clips;
  const [suggest, setSuggest] = useState<string[]>([]);
  const [suggestIndex, setSuggestIndex] = useState(0);
  const timer = useRef(0);

  function search(value: string) {
    setQuery(value);
    const hash = value.match(TRAILING_TAG);
    if (hash) {
      const prefix = hash[1].toLowerCase();
      const names = allTags
        .filter((t) => t.name.toLowerCase().includes(prefix))
        .map((t) => t.name);
      setSuggest(names.slice(0, MAX_SUGGESTIONS));
      setSuggestIndex(0);
    } else {
      setSuggest([]);
    }
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      void load(true, value, tab);
    }, SEARCH_DEBOUNCE_MS);
  }

  function pickSuggestion(name: string) {
    const next = query.replace(
      TRAILING_TAG,
      (all) => `${all.startsWith(" ") ? " " : ""}#${name} `,
    );
    setSuggest([]);
    search(next);
    document.getElementById("q")?.focus();
  }

  function moveSuggestion(delta: number) {
    setSuggestIndex((i) =>
      Math.max(0, Math.min(suggest.length - 1, i + delta)),
    );
  }

  return {
    suggest,
    suggestIndex,
    search,
    clearSearch: () => search(""),
    pickSuggestion,
    moveSuggestion,
    dismissSuggestions: () => setSuggest([]),
  };
}

export type ClipSearch = ReturnType<typeof useClipSearch>;
