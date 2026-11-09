import { useEffect, useRef } from "react";
import type { ClipActions } from "./useClipActions";
import type { ClipSearch } from "./useClipSearch";
import type { Clips } from "./useClips";
import type { WelcomeState } from "./useWelcome";

/** Everything a shortcut can read or trigger. Rebuilt every render, read live. */
export type ShortcutContext = {
  clips: Clips;
  search: ClipSearch;
  actions: ClipActions;
  welcome: WelcomeState;
  settingsOpen: boolean;
  setSettingsOpen: (open: boolean | ((open: boolean) => boolean)) => void;
  toggleMode: () => void;
  toggleTheme: () => void;
};

const focusSearch = () => document.getElementById("q")?.focus();
const isMod = (e: KeyboardEvent) => e.ctrlKey || e.metaKey;
const isModKey = (e: KeyboardEvent, key: string) =>
  isMod(e) && e.key.toLowerCase() === key;

/**
 * Windows and dialogs sit on top of the list, so they get keys first.
 * Returns true when the key is handled or must not reach anything below.
 */
function handleOverlayKeys(e: KeyboardEvent, c: ShortcutContext): boolean {
  if (c.welcome.open) {
    if (e.key === "Enter" || e.key === "Escape") {
      e.preventDefault();
      c.welcome.finish();
    }
    return true;
  }
  if (isMod(e) && e.key === ",") {
    e.preventDefault();
    c.setSettingsOpen((open) => !open);
    return true;
  }
  if (e.key === "Escape") {
    e.preventDefault();
    // Esc steps back one layer at a time and closes the popup last.
    if (c.settingsOpen) {
      c.setSettingsOpen(false);
      focusSearch();
    } else if (c.clips.pickIds.length) c.clips.clearPicks();
    else if (c.actions.secretPrompt) c.actions.cancelSecret();
    else if (c.search.suggest.length) c.search.dismissSuggestions();
    else void c.actions.hideNow();
    return true;
  }
  if (c.settingsOpen) return true;
  if (c.actions.secretPrompt) {
    if (e.key === "Enter") {
      e.preventDefault();
      void c.actions.confirmSecret();
    }
    return true;
  }
  return false;
}

/** Mode, tab, search focus and theme: work from anywhere, even in the tag box. */
function handleGlobalKeys(
  e: KeyboardEvent,
  c: ShortcutContext,
  tagFocus: boolean,
): boolean {
  if (e.key === "Tab" && !tagFocus && c.search.suggest.length === 0) {
    e.preventDefault();
    c.toggleMode();
    return true;
  }
  if (isMod(e) && e.key === "1") {
    e.preventDefault();
    c.clips.changeTab("recent");
    return true;
  }
  if (isMod(e) && e.key === "2") {
    e.preventDefault();
    c.clips.changeTab("pinned");
    return true;
  }
  if (isModKey(e, "k")) {
    e.preventDefault();
    focusSearch();
    return true;
  }
  if (isModKey(e, "d") && e.shiftKey) {
    e.preventDefault();
    c.toggleTheme();
    return true;
  }
  return false;
}

/** Cursor movement, checking clips, undo and the `#tag` suggestion list. */
function handleListKeys(e: KeyboardEvent, c: ShortcutContext): boolean {
  const { clips, search, actions } = c;
  if (isModKey(e, "z") && actions.undoId != null) {
    e.preventDefault();
    actions.undoDelete();
    return true;
  }
  if (isMod(e) && e.code === "Space") {
    e.preventDefault();
    if (clips.current) clips.togglePick(clips.current.id);
    return true;
  }
  if (isModKey(e, "a") && !clips.query.trim()) {
    e.preventDefault();
    clips.selectAll();
    return true;
  }
  if (e.shiftKey && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
    e.preventDefault();
    clips.moveSelection(
      clips.selected + (e.key === "ArrowDown" ? 1 : -1),
      true,
    );
    return true;
  }
  if (e.key === "PageDown" || e.key === "PageUp") {
    e.preventDefault();
    clips.moveSelection(clips.selected + (e.key === "PageDown" ? 6 : -6));
    return true;
  }
  if ((e.key === "Home" || e.key === "End") && !clips.query) {
    e.preventDefault();
    clips.moveSelection(e.key === "Home" ? 0 : clips.cards.length - 1);
    return true;
  }
  if (search.suggest.length && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
    e.preventDefault();
    search.moveSuggestion(e.key === "ArrowDown" ? 1 : -1);
    return true;
  }
  if (search.suggest.length && e.key === "Enter") {
    e.preventDefault();
    const name = search.suggest[search.suggestIndex];
    if (name) search.pickSuggestion(name);
    return true;
  }
  return false;
}

/** Pin, tag, mark secret and delete: act on the selected (or checked) clips. */
function handleClipKeys(e: KeyboardEvent, c: ShortcutContext): boolean {
  const { clips, actions } = c;
  if (isModKey(e, "p")) {
    e.preventDefault();
    if (clips.pickIds.length) actions.pinPicked();
    else void actions.togglePin();
    return true;
  }
  if (isModKey(e, "t")) {
    e.preventDefault();
    document.getElementById("add-tag")?.focus();
    return true;
  }
  if (isModKey(e, "l")) {
    e.preventDefault();
    actions.askSecret();
    return true;
  }
  if (e.key === "Delete" && document.activeElement?.id === "q") {
    // With text after the caret, Del edits the query instead of a clip.
    const input = document.activeElement as HTMLInputElement;
    const atEnd =
      input.selectionStart === input.value.length &&
      input.selectionEnd === input.value.length;
    if (!clips.pickIds.length && !atEnd) return true;
    e.preventDefault();
    if (clips.pickIds.length) actions.deletePicked();
    else actions.askDelete();
    return true;
  }
  return false;
}

/** Plain arrows and Enter only apply while the search box has focus. */
function handleSearchBoxKeys(e: KeyboardEvent, c: ShortcutContext) {
  if (document.activeElement?.id !== "q") return;
  if (e.key === "ArrowDown") {
    e.preventDefault();
    c.clips.moveSelection(c.clips.selected + 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    c.clips.moveSelection(c.clips.selected - 1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    void c.actions.copy(undefined, true);
  }
}

function onKeyDown(e: KeyboardEvent, c: ShortcutContext) {
  const tagFocus = document.activeElement?.id === "add-tag";
  if (handleOverlayKeys(e, c)) return;
  if (handleGlobalKeys(e, c, tagFocus)) return;
  if (tagFocus) return;
  if (handleListKeys(e, c)) return;
  if (handleClipKeys(e, c)) return;
  handleSearchBoxKeys(e, c);
}

/** One window-level key listener, attached once, always reading the latest state. */
export function useShortcuts(context: ShortcutContext) {
  const ref = useRef(context);
  ref.current = context;
  useEffect(() => {
    const listener = (e: KeyboardEvent) => onKeyDown(e, ref.current);
    window.addEventListener("keydown", listener);
    return () => window.removeEventListener("keydown", listener);
  }, []);
}
